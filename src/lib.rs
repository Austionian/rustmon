use anyhow::{Context, Result};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use dotenv::dotenv;
use mongodb::{bson::doc, options::ClientOptions, Client};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;

pub async fn startup() -> Result<Router> {
    dotenv().ok();
    // Connect to the db.
    // Parse a connection string into an options struct.
    let mut client_options = ClientOptions::parse(format!(
        "mongodb+srv://{}:{}@cluster0.n7uefol.mongodb.net/?retryWrites=true&w=majority",
        std::env::var("MONGO_USERNAME").context("Failed to get db username")?,
        std::env::var("MONGO_PASSWORD").context("Failed to get db password")?,
    ))
    .await
    .context("Failed to parse mongo client options.")?;

    // Manually set an option, the app name.
    client_options.app_name = Some("My App".to_string());

    // Get a handle to the deployment.
    let client = Client::with_options(client_options).context("Failed to create db client.")?;

    // build our application with a route
    Ok(Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/users", post(create_user))
        .layer(ServiceBuilder::new().layer(CompressionLayer::new()))
        .layer(TraceLayer::new_for_http())
        .with_state(client))
}

#[derive(Deserialize, Serialize, Clone)]
struct Person {
    name: String,
}

// basic handler that responds with a static string
async fn root(State(client): State<Client>, payload: Query<Person>) -> Result<String, AppError> {
    let db = client.database("test_it_out");
    let collection = db.collection::<Person>("user");
    let filter = doc! { "name": payload.name.clone() };
    let cursor = collection
        .find_one(filter, None)
        .await
        .context("Failed to get collection.")?;

    let name = if let Some(person) = cursor {
        person.name
    } else {
        "None".into()
    };
    // Iterate over the results of the cursor if not using find_one.
    // while let Some(person) = cursor
    //     .try_next()
    //     .await
    //     .context("Failed to move cursor forward")?
    // {
    //     println!("name: {}", person.name);
    // }

    Ok(format!("Hello, {}!", name))
}

async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    payload: Query<Person>,
    State(client): State<Client>,
) -> Result<(StatusCode, Json<Person>), AppError> {
    // insert your application logic here
    let user = Person {
        name: payload.name.clone(),
    };

    let collection = client.database("test_it_out").collection("user");

    let handle = tokio::task::spawn(async move {
        collection
            .insert_one(doc! {"name": payload.name.clone()}, None)
            .await
    });

    tokio::time::timeout(Duration::from_secs(5), handle).await???;

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    Ok((StatusCode::CREATED, Json(user)))
}

struct AppError(anyhow::Error);

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        Self(value.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}
