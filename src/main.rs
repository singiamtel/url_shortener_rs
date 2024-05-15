mod models;
mod schema;

use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};

use diesel::ExpressionMethods;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection, QueryDsl, RunQueryDsl,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
type DB = diesel::pg::Pg;

use dotenvy::dotenv;
use nanoid::nanoid;
use std::env;

type DbPool = Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
struct AppState {
    db: DbPool,
}

#[derive(Deserialize)]
struct CreateRequest {
    url: String,
}

#[derive(Serialize)]
struct CreateSuccess {
    url: String,
}

#[derive(Serialize)]
struct CreateError {
    message: String,
}

enum CreateResponse {
    Success(CreateSuccess),
    Error(CreateError),
}

impl IntoResponse for CreateResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Success(success) => {
                let res = axum::Json(success).into_response();
                (axum::http::StatusCode::CREATED, res).into_response()
            }
            Self::Error(error) => {
                let res = axum::Json(error).into_response();
                (axum::http::StatusCode::BAD_REQUEST, res).into_response()
            }
        }
    }
}

async fn append_to_db(db: DbPool, url: String, short: String) -> Result<String, CreateError> {
    use self::models::NewUrl;
    use self::schema::url;

    let mut conn = db.get().unwrap();

    let awd = diesel::insert_into(url::table)
        .values(&NewUrl {
            name: &url,
            short_url: &short,
            created_by: "user",
        })
        .execute(&mut conn);

    match awd {
        Ok(_) => Ok(short),
        Err(_) => Err(CreateError {
            message: "Failed to create short url".to_string(),
        }),
    }
}

#[derive(Serialize)]
struct FetchError {
    message: String,
}

async fn get_url(db: DbPool, short: String) -> Result<String, FetchError> {
    use schema::url::dsl::{short_url, url};

    let mut conn = db.get().unwrap();

    let result = url
        .filter(short_url.eq(short))
        .first::<models::Url>(&mut conn);

    match result {
        Ok(res) => Ok(res.name),
        Err(_) => Err(FetchError {
            message: "Failed to get url".to_string(),
        }),
    }
}

async fn create_url(
    State(app_state): State<AppState>,
    req: axum::Json<CreateRequest>,
) -> CreateResponse {
    let result = append_to_db(app_state.db, req.url.clone(), nanoid!(6)).await;
    match result {
        Ok(shortened) => CreateResponse::Success(CreateSuccess { url: shortened }),
        Err(e) => CreateResponse::Error(e),
    }
}

#[derive(Serialize)]
struct RedirSuccess {
    url: String,
}

enum RedirResponse {
    Success(RedirSuccess),
    Error(FetchError),
}

impl IntoResponse for RedirResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Success(success) => Redirect::to(&success.url).into_response(),
            Self::Error(error) => {
                let res = axum::Json(error).into_response();
                (axum::http::StatusCode::BAD_REQUEST, res).into_response()
            }
        }
    }
}

async fn get_url_handler(
    State(app_state): State<AppState>,
    Path(short): Path<String>,
) -> RedirResponse {
    let result = get_url(app_state.db, short).await;
    match result {
        Ok(url) => RedirResponse::Success(RedirSuccess { url }),
        Err(e) => RedirResponse::Error(e),
    }
}

fn cleanup_old_links(db: &DbPool, margin: chrono::Duration) -> Result<String, FetchError> {
    use schema::url::dsl::{created_at, url};
    let mut conn = db.get().unwrap();
    let threshold = chrono::Utc::now().naive_utc() - margin;
    let result = diesel::delete(url.filter(created_at.lt(threshold))).execute(&mut conn);
    match result {
        Ok(_) => Ok("Deleted".to_string()),
        Err(_) => Err(FetchError {
            message: "Failed to delete url".to_string(),
        }),
    }
}

fn run_db_migration(conn: &mut impl MigrationHarness<DB>) {
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");
    println!("Migrations run successfully");
}

use std::error::Error;
use std::time::Duration;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let app_state = AppState {
        db: Pool::builder()
            .connection_timeout(Duration::from_secs(3))
            .build(ConnectionManager::<PgConnection>::new(database_url))?,
    };

    let mut conn = app_state
        .db
        .get_timeout(Duration::from_secs(5))
        .expect("Failed to get connection");
    run_db_migration(&mut conn);

    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/create", post(create_url))
        .route("/:short", get(get_url_handler))
        .with_state(app_state.clone());

    tokio::spawn(async move {
        loop {
            println!("Cleaning up old links");
            match cleanup_old_links(&app_state.db, chrono::Duration::days(30)) {
                Ok(_) => (),
                Err(e) => eprintln!("Error cleaning old links: {}", e.message),
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });

    let listener = tokio::net::TcpListener::bind(format!(":::{port}"))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
