//! Голосовые каналы и звонки в ЛС: кто где и пересылка сигналов. Нужен PostgreSQL из ../.env.

mod common;

use std::time::Duration;

use axum::{Router, http::StatusCode};
use common::{Ws, call, register_user, spawn_instance};
use nexus_server::protocol::{ClientMessage, ServerMessage, VoiceFlags};
use serde_json::json;

/// Сервер с владельцем и участником: (owner, owner_id, member, member_id, text_id, voice_id).
async fn setup(app: &Router) -> (String, i64, String, i64, i64, i64) {
    let (owner, owner_id) = register_user(app).await;
    let (member, member_id) = register_user(app).await;
    let r = call(
        app,
        "POST",
        "/api/servers",
        Some(json!({ "name": "Voice" })),
        Some(&owner),
        None,
    )
    .await;
    let sid = r.body["id"].as_i64().unwrap();
    let r = call(
        app,
        "GET",
        &format!("/api/servers/{sid}/channels"),
        None,
        Some(&owner),
        None,
    )
    .await;
    let id_of = |kind: &str| {
        r.body
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["kind"] == kind)
            .unwrap()["id"]
            .as_i64()
            .unwrap()
    };
    let (text_id, voice_id) = (id_of("text"), id_of("voice"));
    let r = call(
        app,
        "POST",
        &format!("/api/servers/{sid}/invites"),
        None,
        Some(&owner),
        None,
    )
    .await;
    let code = r.body["code"].as_str().unwrap();
    call(
        app,
        "POST",
        &format!("/api/invites/{code}/join"),
        None,
        Some(&member),
        None,
    )
    .await;
    (owner, owner_id, member, member_id, text_id, voice_id)
}

/// Ждёт подходящее событие, пропуская остальные (присутствие, печатает и т.п.).
async fn expect(ws: &mut Ws, pred: impl Fn(&ServerMessage) -> bool) -> ServerMessage {
    loop {
        let msg = ws.next().await;
        if pred(&msg) {
            return msg;
        }
    }
}

fn is_error(m: &ServerMessage) -> bool {
    matches!(m, ServerMessage::Error { .. })
}

#[tokio::test]
async fn voice_channel_join_signal_flags_and_leave() {
    // Два экземпляра: всё, что касается голоса, должно ходить между ними.
    let (app1, url1) = spawn_instance().await;
    let (_app2, url2) = spawn_instance().await;
    let (owner, owner_id, member, member_id, text_id, voice_id) = setup(&app1).await;
    let (outsider, _) = register_user(&app1).await;

    let mut owner_ws = Ws::connect(&url1, &owner).await;
    let mut member_ws = Ws::connect(&url2, &member).await;
    let mut outsider_ws = Ws::connect(&url1, &outsider).await;

    // В текстовый канал голосом не войти; посторонний не войдёт и в голосовой.
    member_ws
        .send(&ClientMessage::VoiceJoin {
            channel_id: text_id,
        })
        .await;
    match expect(&mut member_ws, is_error).await {
        ServerMessage::Error { message } => assert_eq!(message, "not a voice channel"),
        _ => unreachable!(),
    }
    outsider_ws
        .send(&ClientMessage::VoiceJoin {
            channel_id: voice_id,
        })
        .await;
    assert!(matches!(
        expect(&mut outsider_ws, is_error).await,
        ServerMessage::Error { .. }
    ));

    // Владелец входит — участник видит это через другой экземпляр.
    owner_ws
        .send(&ClientMessage::VoiceJoin {
            channel_id: voice_id,
        })
        .await;
    let owner_conn = owner_ws.hello_conn_id;
    match expect(&mut member_ws, |m| {
        matches!(m, ServerMessage::VoiceState { .. })
    })
    .await
    {
        ServerMessage::VoiceState { participant } => {
            assert_eq!(
                (participant.channel_id, participant.user_id),
                (voice_id, owner_id)
            );
            assert_eq!(participant.connection_id, owner_conn);
            assert_eq!(participant.flags, VoiceFlags::default());
        }
        _ => unreachable!(),
    }
    let r = call(&app1, "GET", "/api/voice", None, Some(&member), None).await;
    assert_eq!(r.body.as_array().unwrap().len(), 1);
    assert_eq!(r.body[0]["userId"], owner_id);
    let r = call(&app1, "GET", "/api/voice", None, Some(&outsider), None).await;
    assert_eq!(r.body.as_array().unwrap().len(), 0, "outsider sees nothing");

    // Сигналы ходят только между участниками одного голосового канала.
    owner_ws
        .send(&ClientMessage::VoiceSignal {
            to_user_id: member_id,
            data: json!({ "sdp": "x" }),
        })
        .await;
    assert!(
        matches!(
            expect(&mut owner_ws, is_error).await,
            ServerMessage::Error { .. }
        ),
        "member not in voice yet"
    );

    member_ws
        .send(&ClientMessage::VoiceJoin {
            channel_id: voice_id,
        })
        .await;
    expect(&mut owner_ws, |m| matches!(m, ServerMessage::VoiceState { participant } if participant.user_id == member_id)).await;

    let offer = json!({ "description": { "type": "offer", "sdp": "v=0 ..." } });
    owner_ws
        .send(&ClientMessage::VoiceSignal {
            to_user_id: member_id,
            data: offer.clone(),
        })
        .await;
    match expect(&mut member_ws, |m| {
        matches!(m, ServerMessage::VoiceSignal { .. })
    })
    .await
    {
        ServerMessage::VoiceSignal { from_user_id, data } => {
            assert_eq!(from_user_id, owner_id);
            assert_eq!(data, offer);
        }
        _ => unreachable!(),
    }
    // Описание сессии с видео больше лимита NOTIFY (8000 байт) — должно пройти через таблицу.
    let big = json!({ "sdp": "a".repeat(20_000) });
    owner_ws
        .send(&ClientMessage::VoiceSignal {
            to_user_id: member_id,
            data: big.clone(),
        })
        .await;
    match expect(&mut member_ws, |m| {
        matches!(m, ServerMessage::VoiceSignal { .. })
    })
    .await
    {
        ServerMessage::VoiceSignal { data, .. } => assert_eq!(data, big),
        _ => unreachable!(),
    }
    let huge = json!({ "sdp": "a".repeat(70_000) });
    owner_ws
        .send(&ClientMessage::VoiceSignal {
            to_user_id: member_id,
            data: huge,
        })
        .await;
    assert!(matches!(
        expect(&mut owner_ws, is_error).await,
        ServerMessage::Error { .. }
    ));

    // Флаги (микрофон, камера, экран) видны остальным.
    let flags = VoiceFlags {
        muted: true,
        deafened: false,
        camera: true,
        screen: true,
    };
    member_ws
        .send(&ClientMessage::VoiceSetFlags { flags })
        .await;
    match expect(&mut owner_ws, |m| matches!(m, ServerMessage::VoiceState { participant } if participant.user_id == member_id && participant.flags.muted)).await {
        ServerMessage::VoiceState { participant } => assert_eq!(participant.flags, flags),
        _ => unreachable!(),
    }

    // В голосовой канал нельзя писать сообщения.
    let r = call(
        &app1,
        "POST",
        &format!("/api/channels/{voice_id}/messages"),
        Some(json!({ "content": "hi" })),
        Some(&owner),
        None,
    )
    .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);

    // Закрытая вкладка = выход из голоса.
    drop(owner_ws);
    expect(&mut member_ws, |m| matches!(m, ServerMessage::VoiceLeft { user_id, channel_id } if *user_id == owner_id && *channel_id == voice_id)).await;

    // Посторонний за всё время не получил ни одного голосового события.
    while let Some(m) = outsider_ws.next_within(Duration::from_millis(300)).await {
        assert!(
            !matches!(
                m,
                ServerMessage::VoiceState { .. }
                    | ServerMessage::VoiceLeft { .. }
                    | ServerMessage::VoiceSignal { .. }
            ),
            "outsider got {m:?}"
        );
    }
}

#[tokio::test]
async fn second_tab_takes_over_and_dm_calls_work() {
    let (app, url) = spawn_instance().await;
    let (alice, alice_id, bob, _bob_id, _text, voice_id) = setup(&app).await;

    // Вход из второй вкладки переносит голос туда (одна сессия голоса на пользователя).
    let mut tab1 = Ws::connect(&url, &alice).await;
    let mut tab2 = Ws::connect(&url, &alice).await;
    let mut bob_ws = Ws::connect(&url, &bob).await;
    tab1.send(&ClientMessage::VoiceJoin {
        channel_id: voice_id,
    })
    .await;
    expect(&mut bob_ws, |m| matches!(m, ServerMessage::VoiceState { participant } if participant.connection_id == tab1.hello_conn_id)).await;
    tab2.send(&ClientMessage::VoiceJoin {
        channel_id: voice_id,
    })
    .await;
    expect(&mut bob_ws, |m| matches!(m, ServerMessage::VoiceState { participant } if participant.connection_id == tab2.hello_conn_id)).await;
    // Старая вкладка больше не управляет голосом: её выход ничего не меняет.
    tab1.send(&ClientMessage::VoiceLeave).await;
    drop(tab1);
    let r = call(&app, "GET", "/api/voice", None, Some(&bob), None).await;
    assert_eq!(r.body[0]["connectionId"], tab2.hello_conn_id);

    // Звонок в ЛС: Алиса переходит из голосового канала в диалог с Бобом.
    let r = call(&app, "GET", "/api/me", None, Some(&bob), None).await;
    let bob_name = r.body["username"].as_str().unwrap().to_string();
    let r = call(
        &app,
        "POST",
        "/api/dms",
        Some(json!({ "username": bob_name })),
        Some(&alice),
        None,
    )
    .await;
    let dm_id = r.body["id"].as_i64().unwrap();
    tab2.send(&ClientMessage::VoiceJoin { channel_id: dm_id })
        .await;
    expect(&mut bob_ws, |m| matches!(m, ServerMessage::VoiceLeft { user_id, channel_id } if *user_id == alice_id && *channel_id == voice_id)).await;
    match expect(&mut bob_ws, |m| {
        matches!(m, ServerMessage::VoiceState { .. })
    })
    .await
    {
        ServerMessage::VoiceState { participant } => assert_eq!(participant.channel_id, dm_id),
        _ => unreachable!(),
    }
    tab2.send(&ClientMessage::VoiceLeave).await;
    expect(
        &mut bob_ws,
        |m| matches!(m, ServerMessage::VoiceLeft { channel_id, .. } if *channel_id == dm_id),
    )
    .await;
}
