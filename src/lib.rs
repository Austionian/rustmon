use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use mongodb::{bson::doc, options::ClientOptions, Client};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;

pub async fn startup() -> Result<Router> {
    // Connect to the db.
    // Parse a connection string into an options struct.
    let mut client_options = ClientOptions::parse("mongodb://localhost:27017")
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

#[derive(serde::Deserialize)]
struct Person {
    name: String,
}

// basic handler that responds with a static string
async fn root(State(client): State<Client>) -> Result<String, AppError> {
    let db = client.database("test_it_out");
    let collection = db.collection::<Person>("user");
    let filter = doc! { "name": "Austin Rooks" };
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
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    // insert your application logic here
    let user = User {
        id: 1337,
        username: payload.username,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
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
