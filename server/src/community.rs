use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use chrono::{DateTime, TimeZone, Utc};
use diesel::{
    ExpressionMethods, QueryableByName,
    sql_types::{BigInt, Bool, Int4, Nullable, Text, Timestamptz},
};
use diesel_async::{AsyncConnection, RunQueryDsl};

use crate::{
    AppState,
    auth::AuthUser,
    error::{AppError, AppResult},
    events::{Notice, notify},
    permissions::{self, CREATE_THREADS, KICK_MEMBERS, MANAGE_CHANNELS, MANAGE_ROLES},
    protocol::{
        ChannelKind, CreateFromTemplateRequest, CreateOnboardingQuestionRequest,
        CreateServerEventRequest, CreateThreadTagRequest, OnboardingQuestion, PruneResponse,
        ServerEvent, ServerTemplate, TemplateChannel, TemplateRole, ThreadTag,
    },
    schema::member_roles,
    servers::role_in,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/servers/{id}/onboarding",
            get(onboarding_list).post(onboarding_create),
        )
        .route(
            "/api/servers/{id}/onboarding/{question_id}",
            delete(onboarding_delete),
        )
        .route(
            "/api/servers/{id}/onboarding/{question_id}/answer",
            post(onboarding_answer),
        )
        .route(
            "/api/servers/{id}/events",
            get(events_list).post(events_create),
        )
        .route("/api/servers/{id}/events/{event_id}", delete(events_delete))
        .route(
            "/api/servers/{id}/events/{event_id}/interest",
            post(events_interest),
        )
        .route("/api/servers/{id}/tags", get(tags_list).post(tags_create))
        .route("/api/servers/{id}/tags/{tag_id}", delete(tags_delete))
        .route("/api/servers/{id}/template", get(template_get))
        .route("/api/servers/template", post(template_create))
        .route("/api/servers/{id}/prune", post(prune))
        .route(
            "/api/channels/{id}/threads",
            get(threads_list).post(threads_create),
        )
        .route(
            "/api/messages/{id}/replies",
            get(thread_replies),
        )
        .route(
            "/api/messages/{id}/thread",
            axum::routing::patch(thread_update),
        )
}

#[derive(QueryableByName)]
struct OnboardingRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = Text)]
    question: String,
    #[diesel(sql_type = BigInt)]
    role_id: i64,
    #[diesel(sql_type = Int4)]
    position: i32,
    #[diesel(sql_type = Bool)]
    answered: bool,
}

async fn onboarding_list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
) -> AppResult<Json<Vec<OnboardingQuestion>>> {
    let mut conn = state.pool.get().await?;
    role_in(&mut conn, server_id, user_id).await?;
    let rows: Vec<OnboardingRow> = diesel::sql_query(
        "SELECT q.id, q.question, q.role_id, q.position,
                EXISTS(SELECT 1 FROM onboarding_answers a WHERE a.question_id = q.id AND a.user_id = $2) AS answered
         FROM onboarding_questions q WHERE q.server_id = $1 ORDER BY q.position, q.id",
    )
    .bind::<BigInt, _>(server_id)
    .bind::<BigInt, _>(user_id)
    .load(&mut conn)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| OnboardingQuestion {
                id: r.id,
                server_id,
                question: r.question,
                role_id: r.role_id,
                position: r.position,
                answered: r.answered,
            })
            .collect(),
    ))
}

async fn onboarding_create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
    Json(req): Json<CreateOnboardingQuestionRequest>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, MANAGE_ROLES).await?;
    let question = req.question.trim();
    if question.is_empty() || question.chars().count() > 500 {
        return Err(AppError::bad_request("invalid question"));
    }
    let role: Option<i64> = diesel::sql_query(
        "SELECT id FROM roles WHERE id = $1 AND server_id = $2 AND is_default = FALSE",
    )
    .bind::<BigInt, _>(req.role_id)
    .bind::<BigInt, _>(server_id)
    .load(&mut conn)
    .await?
    .into_iter()
    .next()
    .map(|r: IdRow| r.id);
    if role.is_none() {
        return Err(AppError::bad_request("unknown role"));
    }
    diesel::sql_query(
        "INSERT INTO onboarding_questions (server_id, question, role_id, position) \
         VALUES ($1, $2, $3, COALESCE((SELECT max(position)+1 FROM onboarding_questions WHERE server_id=$1),0))",
    )
    .bind::<BigInt, _>(server_id).bind::<Text, _>(question).bind::<BigInt, _>(req.role_id)
    .execute(&mut conn).await?;
    Ok(StatusCode::CREATED)
}

async fn onboarding_delete(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, question_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, MANAGE_ROLES).await?;
    diesel::sql_query("DELETE FROM onboarding_questions WHERE id = $1 AND server_id = $2")
        .bind::<BigInt, _>(question_id)
        .bind::<BigInt, _>(server_id)
        .execute(&mut conn)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn onboarding_answer(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, question_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    role_in(&mut conn, server_id, user_id).await?;
    let role: Option<i64> = diesel::sql_query(
        "SELECT role_id FROM onboarding_questions WHERE id = $1 AND server_id = $2",
    )
    .bind::<BigInt, _>(question_id)
    .bind::<BigInt, _>(server_id)
    .load(&mut conn)
    .await?
    .into_iter()
    .next()
    .map(|r: IdRow| r.id);
    let Some(role_id) = role else {
        return Err(AppError::not_found("question"));
    };
    diesel::sql_query("INSERT INTO onboarding_answers (question_id, user_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
        .bind::<BigInt, _>(question_id).bind::<BigInt, _>(user_id).execute(&mut conn).await?;
    diesel::insert_into(member_roles::table)
        .values((
            member_roles::server_id.eq(server_id),
            member_roles::user_id.eq(user_id),
            member_roles::role_id.eq(role_id),
        ))
        .on_conflict_do_nothing()
        .execute(&mut conn)
        .await?;
    notify(&mut conn, &Notice::RolesChanged { server_id }).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(QueryableByName)]
struct IdRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
}

#[derive(QueryableByName)]
struct EventRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    description: String,
    #[diesel(sql_type = Timestamptz)]
    starts_at: DateTime<Utc>,
    #[diesel(sql_type = Nullable<BigInt>)]
    channel_id: Option<i64>,
    #[diesel(sql_type = BigInt)]
    creator_id: i64,
    #[diesel(sql_type = Int4)]
    interested_count: i32,
    #[diesel(sql_type = Bool)]
    interested: bool,
}

fn event_dto(r: EventRow, server_id: i64) -> ServerEvent {
    ServerEvent {
        id: r.id,
        server_id,
        name: r.name,
        description: r.description,
        starts_at: r.starts_at.timestamp_millis(),
        channel_id: r.channel_id,
        creator_id: r.creator_id,
        interested_count: r.interested_count,
        interested: r.interested,
    }
}

async fn send_due_reminders(
    conn: &mut diesel_async::AsyncPgConnection,
    server_id: i64,
) -> AppResult<()> {
    #[derive(QueryableByName)]
    struct Due {
        #[diesel(sql_type = BigInt)]
        event_id: i64,
        #[diesel(sql_type = Text)]
        name: String,
        #[diesel(sql_type = BigInt)]
        creator_id: i64,
        #[diesel(sql_type = Nullable<BigInt>)]
        channel_id: Option<i64>,
    }
    let due: Vec<Due> = diesel::sql_query(
        "SELECT e.id AS event_id, e.name, e.creator_id, e.channel_id FROM server_events e \
         WHERE e.server_id = $1 AND e.starts_at > now() AND e.starts_at < now() + interval '1 hour' \
         AND EXISTS (SELECT 1 FROM event_interests i WHERE i.event_id = e.id AND i.reminded_at IS NULL)",
    ).bind::<BigInt, _>(server_id).load(conn).await?;
    let system_channel: Option<i64> =
        diesel::sql_query("SELECT system_channel_id FROM servers WHERE id = $1")
            .bind::<BigInt, _>(server_id)
            .load(conn)
            .await?
            .into_iter()
            .next()
            .and_then(|r: NullableIdRow| r.server_id);
    for e in due {
        if let Some(channel_id) = e.channel_id.or(system_channel) {
            let text = format!("Событие «{}» начнётся через час", e.name);
            crate::messages::post_system(
                conn,
                server_id,
                channel_id,
                e.creator_id,
                "eventReminder",
                Some(&e.name),
                &text,
            )
            .await?;
        }
        diesel::sql_query("UPDATE event_interests SET reminded_at = now() WHERE event_id = $1 AND reminded_at IS NULL")
            .bind::<BigInt, _>(e.event_id).execute(conn).await?;
    }
    Ok(())
}

async fn events_list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
) -> AppResult<Json<Vec<ServerEvent>>> {
    let mut conn = state.pool.get().await?;
    role_in(&mut conn, server_id, user_id).await?;
    send_due_reminders(&mut conn, server_id).await?;
    let rows: Vec<EventRow> = diesel::sql_query(
        "SELECT e.id, e.name, e.description, e.starts_at, e.channel_id, e.creator_id,
                (SELECT count(*)::int FROM event_interests i WHERE i.event_id = e.id) AS interested_count,
                EXISTS(SELECT 1 FROM event_interests i WHERE i.event_id = e.id AND i.user_id = $2) AS interested
         FROM server_events e WHERE e.server_id = $1 AND e.starts_at > now() - interval '1 day'
         ORDER BY e.starts_at",
    ).bind::<BigInt, _>(server_id).bind::<BigInt, _>(user_id).load(&mut conn).await?;
    Ok(Json(
        rows.into_iter().map(|r| event_dto(r, server_id)).collect(),
    ))
}

async fn events_create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
    Json(req): Json<CreateServerEventRequest>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, MANAGE_CHANNELS).await?;
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(AppError::bad_request("invalid event name"));
    }
    let starts = Utc
        .timestamp_millis_opt(req.starts_at)
        .single()
        .ok_or_else(|| AppError::bad_request("invalid time"))?;
    let channel_id = req.channel_id.flatten();
    if let Some(id) = channel_id {
        let server: Option<i64> = diesel::sql_query("SELECT server_id FROM channels WHERE id = $1")
            .bind::<BigInt, _>(id)
            .load(&mut conn)
            .await?
            .into_iter()
            .next()
            .and_then(|r: NullableIdRow| r.server_id);
        if server != Some(server_id) {
            return Err(AppError::bad_request("unknown channel"));
        }
    }
    diesel::sql_query(
        "INSERT INTO server_events (server_id, name, description, starts_at, channel_id, creator_id) VALUES ($1,$2,$3,$4,$5,$6)",
    ).bind::<BigInt, _>(server_id).bind::<Text, _>(name).bind::<Text, _>(&req.description)
     .bind::<Timestamptz, _>(starts).bind::<Nullable<BigInt>, _>(channel_id).bind::<BigInt, _>(user_id)
     .execute(&mut conn).await?;
    let _ = req.reminder;
    Ok(StatusCode::CREATED)
}

#[derive(QueryableByName)]
struct NullableIdRow {
    #[diesel(sql_type = Nullable<BigInt>)]
    server_id: Option<i64>,
}

async fn events_delete(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, event_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let creator: Option<i64> =
        diesel::sql_query("SELECT creator_id FROM server_events WHERE id=$1 AND server_id=$2")
            .bind::<BigInt, _>(event_id)
            .bind::<BigInt, _>(server_id)
            .load(&mut conn)
            .await?
            .into_iter()
            .next()
            .map(|r: IdRow| r.id);
    let Some(creator_id) = creator else {
        return Err(AppError::not_found("event"));
    };
    let manager = permissions::require_permission(&mut conn, server_id, user_id, MANAGE_CHANNELS)
        .await
        .is_ok();
    if creator_id != user_id && !manager {
        return Err(AppError::forbidden());
    }
    diesel::sql_query("DELETE FROM server_events WHERE id=$1")
        .bind::<BigInt, _>(event_id)
        .execute(&mut conn)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn events_interest(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, event_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    role_in(&mut conn, server_id, user_id).await?;
    let exists: bool = diesel::sql_query(
        "SELECT count(*)::int AS id FROM event_interests WHERE event_id=$1 AND user_id=$2",
    )
    .bind::<BigInt, _>(event_id)
    .bind::<BigInt, _>(user_id)
    .load(&mut conn)
    .await?
    .into_iter()
    .next()
    .map(|r: CountRow| r.id > 0)
    .unwrap_or(false);
    if exists {
        diesel::sql_query("DELETE FROM event_interests WHERE event_id=$1 AND user_id=$2")
            .bind::<BigInt, _>(event_id)
            .bind::<BigInt, _>(user_id)
            .execute(&mut conn)
            .await?;
    } else {
        diesel::sql_query("INSERT INTO event_interests (event_id,user_id) VALUES ($1,$2)")
            .bind::<BigInt, _>(event_id)
            .bind::<BigInt, _>(user_id)
            .execute(&mut conn)
            .await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = Int4)]
    id: i32,
}

#[derive(QueryableByName)]
struct TagRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Nullable<Int4>)]
    color: Option<i32>,
}

async fn tags_list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
) -> AppResult<Json<Vec<ThreadTag>>> {
    let mut conn = state.pool.get().await?;
    role_in(&mut conn, server_id, user_id).await?;
    let rows: Vec<TagRow> = diesel::sql_query(
        "SELECT t.id, t.name, t.color FROM thread_tags t JOIN channels c ON c.id=t.channel_id WHERE c.server_id=$1 ORDER BY t.name",
    ).bind::<BigInt, _>(server_id).load(&mut conn).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| ThreadTag {
                id: r.id,
                channel_id: 0,
                name: r.name,
                color: r.color,
            })
            .collect(),
    ))
}

async fn tags_create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
    Json(req): Json<CreateThreadTagRequest>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, MANAGE_CHANNELS).await?;
    let name = req.name.trim();
    if name.is_empty() || name.chars().count() > 32 {
        return Err(AppError::bad_request("invalid tag"));
    }
    let forum: Option<i64> = diesel::sql_query(
        "SELECT id FROM channels WHERE server_id=$1 AND is_forum=TRUE ORDER BY id LIMIT 1",
    )
    .bind::<BigInt, _>(server_id)
    .load(&mut conn)
    .await?
    .into_iter()
    .next()
    .map(|r: IdRow| r.id);
    let Some(channel_id) = forum else {
        return Err(AppError::bad_request("no forum channel"));
    };
    diesel::sql_query(
        "INSERT INTO thread_tags (channel_id,name,color) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
    )
    .bind::<BigInt, _>(channel_id)
    .bind::<Text, _>(name)
    .bind::<Nullable<Int4>, _>(req.color.flatten())
    .execute(&mut conn)
    .await?;
    Ok(StatusCode::CREATED)
}

async fn tags_delete(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path((server_id, tag_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, MANAGE_CHANNELS).await?;
    diesel::sql_query("DELETE FROM thread_tags WHERE id=$1 AND channel_id IN (SELECT id FROM channels WHERE server_id=$2)")
        .bind::<BigInt, _>(tag_id).bind::<BigInt, _>(server_id).execute(&mut conn).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(QueryableByName)]
struct TemplateRow {
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    description: String,
    #[diesel(sql_type = Text)]
    rules: String,
    #[diesel(sql_type = Text)]
    banner_url: String,
}

async fn template_get(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
) -> AppResult<Json<ServerTemplate>> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, MANAGE_CHANNELS).await?;
    let row: TemplateRow =
        diesel::sql_query("SELECT name,description,rules,banner_url FROM servers WHERE id=$1")
            .bind::<BigInt, _>(server_id)
            .load(&mut conn)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::not_found("server"))?;
    #[derive(QueryableByName)]
    struct Ch {
        #[diesel(sql_type = Text)]
        name: String,
        #[diesel(sql_type = Text)]
        kind: String,
        #[diesel(sql_type = Bool)]
        is_forum: bool,
    }
    let channels: Vec<TemplateChannel> = diesel::sql_query(
        "SELECT name,kind,is_forum FROM channels WHERE server_id=$1 ORDER BY position,id",
    )
    .bind::<BigInt, _>(server_id)
    .load(&mut conn)
    .await?
    .into_iter()
    .map(|c: Ch| TemplateChannel {
        name: c.name,
        kind: ChannelKind::parse(&c.kind),
        is_forum: c.is_forum,
    })
    .collect();
    #[derive(QueryableByName)]
    struct R {
        #[diesel(sql_type = Text)]
        name: String,
        #[diesel(sql_type = Nullable<Int4>)]
        color: Option<i32>,
        #[diesel(sql_type = BigInt)]
        permissions: i64,
        #[diesel(sql_type = Int4)]
        position: i32,
        #[diesel(sql_type = Bool)]
        hoist: bool,
        #[diesel(sql_type = Bool)]
        mentionable: bool,
    }
    let roles: Vec<TemplateRole> = diesel::sql_query("SELECT name,color,permissions,position,hoist,mentionable FROM roles WHERE server_id=$1 AND is_default=FALSE ORDER BY position DESC,id")
        .bind::<BigInt, _>(server_id).load(&mut conn).await?.into_iter().map(|r: R| TemplateRole {
            name: r.name, color: r.color, permissions: r.permissions, position: r.position, hoist: r.hoist, mentionable: r.mentionable,
        }).collect();
    Ok(Json(ServerTemplate {
        name: row.name,
        description: row.description,
        rules: row.rules,
        banner_url: row.banner_url,
        channels,
        roles,
    }))
}

async fn template_create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Json(req): Json<CreateFromTemplateRequest>,
) -> AppResult<(StatusCode, Json<crate::protocol::Server>)> {
    let t = req.template;
    let name = crate::servers::validate_server_name(&t.name)?;
    let mut conn = state.pool.get().await?;
    let server = conn.transaction(async move |conn| -> AppResult<crate::protocol::Server> {
        let id: i64 = diesel::sql_query("INSERT INTO servers (name,owner_id,description,rules,banner_url) VALUES ($1,$2,$3,$4,$5) RETURNING id")
            .bind::<Text, _>(&name).bind::<BigInt, _>(user_id).bind::<Text, _>(&t.description)
            .bind::<Text, _>(&t.rules).bind::<Text, _>(&t.banner_url)
            .load(conn).await?.into_iter().next().map(|r: IdRow| r.id).ok_or_else(|| AppError::bad_request("create failed"))?;
        diesel::sql_query("INSERT INTO members (server_id,user_id,role) VALUES ($1,$2,'owner')")
            .bind::<BigInt, _>(id).bind::<BigInt, _>(user_id).execute(conn).await?;
        diesel::sql_query("INSERT INTO roles (server_id,name,permissions,position,is_default) VALUES ($1,'@everyone',0,0,TRUE)")
            .bind::<BigInt, _>(id).execute(conn).await?;
        let mut system = None;
        for (position, c) in t.channels.iter().enumerate() {
            let cid: i64 = diesel::sql_query("INSERT INTO channels (server_id,name,position,kind,is_forum) VALUES ($1,$2,$3,$4,$5) RETURNING id")
                .bind::<BigInt, _>(id).bind::<Text, _>(&c.name).bind::<Int4, _>(position as i32)
                .bind::<Text, _>(c.kind.as_str()).bind::<Bool, _>(c.is_forum)
                .load(conn).await?.into_iter().next().map(|r: IdRow| r.id).ok_or_else(|| AppError::bad_request("channel failed"))?;
            if system.is_none() && c.kind == ChannelKind::Text { system = Some(cid); }
        }
        for r in &t.roles {
            diesel::sql_query("INSERT INTO roles (server_id,name,color,permissions,position,hoist,mentionable) VALUES ($1,$2,$3,$4,$5,$6,$7)")
                .bind::<BigInt, _>(id).bind::<Text, _>(&r.name).bind::<Nullable<Int4>, _>(r.color)
                .bind::<BigInt, _>(r.permissions).bind::<Int4, _>(r.position.clamp(1,1000))
                .bind::<Bool, _>(r.hoist).bind::<Bool, _>(r.mentionable).execute(conn).await?;
        }
        if let Some(system) = system {
            diesel::sql_query("UPDATE servers SET system_channel_id=$1 WHERE id=$2").bind::<BigInt, _>(system).bind::<BigInt, _>(id).execute(conn).await?;
        }
        Ok(crate::protocol::Server {
            id, name, owner_id: user_id, my_role: crate::protocol::Role::Owner,
            my_permissions: permissions::ALL, unread_count: 0, system_channel_id: system,
            afk_channel_id: None, description: t.description, rules: t.rules, banner_url: t.banner_url, announcement_channel_id: None,
            boost_count: 0, boost_level: 0,
        })
    }).await?;
    Ok((StatusCode::CREATED, Json(server)))
}

async fn prune(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(server_id): Path<i64>,
    Query(q): Query<PruneQuery>,
) -> AppResult<Json<PruneResponse>> {
    let mut conn = state.pool.get().await?;
    permissions::require_permission(&mut conn, server_id, user_id, KICK_MEMBERS).await?;
    let days = q.days.unwrap_or(30).clamp(1, 365);
    let removed: i32 = diesel::sql_query(
        "DELETE FROM members WHERE server_id=$1 AND role='member' AND user_id NOT IN \
         (SELECT author_id FROM messages WHERE created_at > now() - ($2 || ' days')::interval) RETURNING 1 AS id",
    ).bind::<BigInt, _>(server_id).bind::<Text, _>(&days.to_string())
     .load::<IdRow>(&mut conn).await?.len() as i32;
    notify(&mut conn, &Notice::RolesChanged { server_id }).await?;
    Ok(Json(PruneResponse { removed }))
}

#[derive(Deserialize)]
pub struct PruneQuery {
    days: Option<i64>,
}

#[derive(QueryableByName)]
struct ThreadRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
}

async fn threads_list(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
) -> AppResult<Json<Vec<crate::protocol::Message>>> {
    let mut conn = state.pool.get().await?;
    let access = crate::servers::channel_access(&mut conn, channel_id, user_id).await?;
    if let Some(server_id) = access.server_id {
        permissions::require_channel_permission(
            &mut conn,
            server_id,
            user_id,
            channel_id,
            permissions::VIEW_CHANNEL,
        )
        .await?;
    }
    let rows: Vec<ThreadRow> = diesel::sql_query(
        "SELECT id, content, author_id, created_at, thread_archived, thread_locked FROM messages WHERE channel_id=$1 AND (thread_root_id IS NULL OR thread_root_id = $1) ORDER BY id DESC LIMIT 100",
    ).bind::<BigInt, _>(channel_id).load(&mut conn).await?;
    let mut out = Vec::new();
    for r in rows {
        let message = crate::messages::load_message(&mut conn, &state.file_key, r.id).await?;
        if let Some(m) = message {
            out.push(m);
        }
    }
    Ok(Json(out))
}

async fn thread_replies(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(message_id): Path<i64>,
) -> AppResult<Json<Vec<crate::protocol::Message>>> {
    let mut conn = state.pool.get().await?;
    // Проверяем доступ к сообщению-корню ветки
    #[derive(QueryableByName)]
    struct ChannelIdRow {
        #[diesel(sql_type = BigInt)]
        channel_id: i64,
    }
    let channel_row: Option<ChannelIdRow> = diesel::sql_query("SELECT channel_id FROM messages WHERE id=$1")
        .bind::<BigInt, _>(message_id)
        .load(&mut conn)
        .await?
        .into_iter()
        .next();
    let Some(channel_row) = channel_row else {
        return Err(AppError::not_found("thread root"));
    };
    let channel_id = channel_row.channel_id;
    let access = crate::servers::channel_access(&mut conn, channel_id, user_id).await?;
    if let Some(server_id) = access.server_id {
        permissions::require_channel_permission(
            &mut conn,
            server_id,
            user_id,
            channel_id,
            permissions::VIEW_CHANNEL,
        )
        .await?;
    }
    let rows: Vec<ThreadRow> = diesel::sql_query(
        "SELECT id, content, author_id, created_at, thread_archived, thread_locked FROM messages WHERE thread_root_id=$1 ORDER BY created_at ASC",
    )
    .bind::<BigInt, _>(message_id)
    .load(&mut conn)
    .await?;
    let mut out = Vec::new();
    for r in rows {
        if let Some(m) = crate::messages::load_message(&mut conn, &state.file_key, r.id).await? {
            out.push(m);
        }
    }
    Ok(Json(out))
}

#[derive(Deserialize)]
pub struct CreateThreadRequest {
    content: String,
}

async fn threads_create(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(channel_id): Path<i64>,
    Json(req): Json<CreateThreadRequest>,
) -> AppResult<(StatusCode, Json<crate::protocol::Message>)> {
    let mut conn = state.pool.get().await?;
    let access = crate::servers::channel_access(&mut conn, channel_id, user_id).await?;
    if let Some(server_id) = access.server_id {
        permissions::require_channel_permission(
            &mut conn,
            server_id,
            user_id,
            channel_id,
            permissions::VIEW_CHANNEL | CREATE_THREADS,
        )
        .await?;
    }
    let content = crate::messages::validate(&req.content, false)?;
    let id: i64 = diesel::sql_query(
        "INSERT INTO messages (channel_id,author_id,content) VALUES ($1,$2,$3) RETURNING id",
    )
    .bind::<BigInt, _>(channel_id)
    .bind::<BigInt, _>(user_id)
    .bind::<Text, _>(&content)
    .load(&mut conn)
    .await?
    .into_iter()
    .next()
    .map(|r: IdRow| r.id)
    .ok_or_else(|| AppError::bad_request("create failed"))?;
    notify(
        &mut conn,
        &Notice::MessageCreated {
            server_id: access.server_id,
            message_id: id,
        },
    )
    .await?;
    let message = crate::messages::load_message(&mut conn, &state.file_key, id)
        .await?
        .ok_or_else(|| AppError::not_found("message"))?;
    Ok((StatusCode::CREATED, Json(message)))
}

#[derive(Deserialize)]
pub struct ThreadUpdateRequest {
    #[serde(default)]
    archived: Option<bool>,
    #[serde(default)]
    locked: Option<bool>,
}

async fn thread_update(
    State(state): State<AppState>,
    AuthUser(user_id): AuthUser,
    Path(message_id): Path<i64>,
    Json(req): Json<ThreadUpdateRequest>,
) -> AppResult<StatusCode> {
    let mut conn = state.pool.get().await?;
    let (channel_id, _author, access) =
        crate::messages::locate(&mut conn, message_id, user_id).await?;
    if let Some(server_id) = access.server_id {
        permissions::require_channel_permission(
            &mut conn,
            server_id,
            user_id,
            channel_id,
            permissions::MANAGE_THREADS,
        )
        .await?;
    }
    if let Some(archived) = req.archived {
        diesel::sql_query("UPDATE messages SET thread_archived=$1 WHERE id=$2")
            .bind::<Bool, _>(archived)
            .bind::<BigInt, _>(message_id)
            .execute(&mut conn)
            .await?;
    }
    if let Some(locked) = req.locked {
        diesel::sql_query("UPDATE messages SET thread_locked=$1 WHERE id=$2")
            .bind::<Bool, _>(locked)
            .bind::<BigInt, _>(message_id)
            .execute(&mut conn)
            .await?;
    }
    notify(
        &mut conn,
        &Notice::MessageUpdated {
            server_id: access.server_id,
            message_id,
        },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

use serde::Deserialize;
