use axum::{
    routing::get,
    http::StatusCode,
    Json, Router,
    extract::State,
};
use realtor_api::calculate_rental_yield;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    db: PgPool,
}

#[derive(Serialize, Deserialize)]
struct ApiResponse {
    message: String,
    status: String,
}

#[tokio::main]
async fn main() {
    println!("🏠 Starting Realtor API server...");

    // Load environment variables
    dotenvy::dotenv().ok();

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");

    // Create database connection pool
    println!("📦 Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    println!("✅ Database connected successfully");

    let state = AppState { db: pool };

    let app = Router::new()
        .route("/", get(health_check))
        .route("/api/health", get(health_check))
        .route("/api/properties", get(get_properties))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    println!("🚀 Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> Json<ApiResponse> {
    Json(ApiResponse {
        message: "Realtor API is running!".to_string(),
        status: "ok".to_string(),
    })
}

async fn get_properties(State(state): State<AppState>) -> Result<Json<Vec<Property>>, StatusCode> {
    let properties = sqlx::query_as!(
        PropertyRow,
        r#"
        SELECT
            id,
            address,
            suburb,
            state as "state: StateEnum",
            bedrooms,
            price,
            weekly_rent,
            latitude,
            longitude
        FROM properties
        ORDER BY id
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        eprintln!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Convert to response format with calculated rental yield
    let response: Vec<Property> = properties
        .into_iter()
        .map(|p| {
            let rental_yield = if let (Some(price), Some(rent)) = (p.price, p.weekly_rent) {
                calculate_rental_yield(price, rent)
            } else {
                None
            };

            Property {
                id: p.id,
                address: p.address,
                suburb: p.suburb,
                state: format!("{:?}", p.state),
                bedrooms: p.bedrooms,
                price: p.price,
                weekly_rent: p.weekly_rent,
                latitude: p.latitude,
                longitude: p.longitude,
                rental_yield,
            }
        })
        .collect();

    Ok(Json(response))
}

#[derive(sqlx::Type, Debug)]
#[sqlx(type_name = "state_enum", rename_all = "UPPERCASE")]
enum StateEnum {
    NSW,
    VIC,
    QLD,
    WA,
    SA,
    TAS,
    ACT,
    NT,
}

#[derive(sqlx::FromRow)]
struct PropertyRow {
    id: i32,
    address: String,
    suburb: String,
    state: StateEnum,
    bedrooms: Option<i32>,
    price: Option<i32>,
    weekly_rent: Option<i32>,
    latitude: Option<rust_decimal::Decimal>,
    longitude: Option<rust_decimal::Decimal>,
}

#[derive(Serialize, Deserialize)]
struct Property {
    id: i32,
    address: String,
    suburb: String,
    state: String,
    bedrooms: Option<i32>,
    price: Option<i32>,
    weekly_rent: Option<i32>,
    latitude: Option<rust_decimal::Decimal>,
    longitude: Option<rust_decimal::Decimal>,
    rental_yield: Option<f32>,
}