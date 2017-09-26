use std::io::Cursor;
use rocket_contrib::json::{JsonValue, Json};
use rocket::request::Request;
use rocket::response::{Response, Responder};
use rocket::http::{Status, ContentType};
use diesel;
use uuid;
use models::user::Error as UserModelError;
use models::user::ErrorKind as UserModelErrorKind;

#[derive(Debug)]
pub struct APIResponse {
    message: Option<String>,
    data: Option<JsonValue>,
    status: Status,
}

impl APIResponse {
    /// Change the message of the `Response`.
    pub fn message(mut self, message: &str) -> APIResponse {
        self.message = Some(message.to_string());
        self
    }

    /// Change the data to the `Response`.
    pub fn data(mut self, data: JsonValue) -> APIResponse {
        self.data = Some(data);
        self
    }
}

impl<'r> Responder<'r> for APIResponse {
    fn respond_to(self, request: &Request) -> Result<Response<'r>, Status> {
        let body = match (self.data, self.message) {
            (Some(data), _) => data,
            (_, Some(message)) => json!({ "message": message }),
            (None, None) => panic!()
        };

        Response::build()
            .status(self.status)
            .sized_body(Cursor::new(body.to_string()))
            .header(ContentType::JSON)
            .ok()
    }
}

impl From<uuid::ParseError> for APIResponse {
    fn from(error: uuid::ParseError) -> Self {
        bad_request()
    }
}

impl From<UserModelError> for APIResponse {
    fn from(error: UserModelError) -> Self {
        match error.kind() {
            &UserModelErrorKind::UserExists(ref user_name) =>
                conflict().message(&format!("{}", error)),
            &UserModelErrorKind::Db(ref db_error) => APIResponse::from(db_error),
            _ => bad_request().message("Something is wrong with the auth token or login details you provided.")
        }
    }
}

impl From<diesel::result::Error> for APIResponse {
    fn from(error: diesel::result::Error) -> Self {
        APIResponse::from(&error)
    }
}

impl<'a> From<&'a diesel::result::Error> for APIResponse {
    fn from(error: &diesel::result::Error) -> Self {
        use diesel::result::Error;
        match error {
            &Error::NotFound => not_found(),
            _ => internal_server_error()
        }
    }
}

pub fn ok() -> APIResponse {
    APIResponse {
        message: Some("Ok".to_string()),
        data: None,
        status: Status::Ok,
    }
}

pub fn created() -> APIResponse {
    APIResponse {
        message: Some("Created".to_string()),
        data: None,
        status: Status::Created,
    }
}

pub fn accepted() -> APIResponse {
    APIResponse {
        message: Some("Accepted".to_string()),
        data: None,
        status: Status::Accepted,
    }
}

pub fn no_content() -> APIResponse {
    APIResponse {
        message: Some("No Content".to_string()),
        data: None,
        status: Status::NoContent,
    }
}


pub fn bad_request() -> APIResponse {
    APIResponse {
        message: Some("Bad Request".to_string()),
        data: None,
        status: Status::BadRequest,
    }
}

pub fn unauthorized() -> APIResponse {
    APIResponse {
        message: Some("Unauthorized".to_string()),
        data: None,
        status: Status::Unauthorized,
    }
}

pub fn forbidden() -> APIResponse {
    APIResponse {
        message: Some("Forbidden".to_string()),
        data: None,
        status: Status::Forbidden,
    }
}

pub fn not_found() -> APIResponse {
    APIResponse {
        message: Some("Not Found".to_string()),
        data: None,
        status: Status::NotFound,
    }
}

pub fn method_not_allowed() -> APIResponse {
    APIResponse {
        message: Some("Method Not Allowed".to_string()),
        data: None,
        status: Status::MethodNotAllowed,
    }
}

pub fn conflict() -> APIResponse {
    APIResponse {
        message: Some("Conflict".to_string()),
        data: None,
        status: Status::Conflict,
    }
}

pub fn unprocessable_entity() -> APIResponse {
    APIResponse {
        message: Some("Unprocessable Entity".to_string()),
        data: None,
        status: Status::UnprocessableEntity,
    }
}

pub fn internal_server_error() -> APIResponse {
    APIResponse {
        message: Some("Internal Server Error".to_string()),
        data: None,
        status: Status::InternalServerError,
    }
}

pub fn service_unavailable() -> APIResponse {
    APIResponse {
        message: Some("Service Unavailable".to_string()),
        data: None,
        status: Status::ServiceUnavailable,
    }
}
