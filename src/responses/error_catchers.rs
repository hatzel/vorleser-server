use rocket_contrib::Value;
use failure::Error;
use rocket::Error as RocketError;
use rocket::response::content;
use responses::APIError;
use rocket::http::Status;
use rocket_contrib::Json;

#[error(400)]
pub fn bad_request(e: RocketError) -> APIError {
    APIError::builder(Status::BadRequest)
        .message("Bad Request!".to_string())
        .build()
}
