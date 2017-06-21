use helpers::db::init_db_pool;
use helpers;
use diesel::prelude::*;
use models::user::UserModel;
use rocket::local::Client;
use rocket::http::{Status, Method, Header, ContentType};
use serde_json::{self, Value};

describe! api_tests {
    before_each {
        let mut pool = init_db_pool();
        let conn = &*pool.get().unwrap();
        conn.execute("TRUNCATE audiobooks, chapters, playstates, users RESTART IDENTITY CASCADE").unwrap();
        let rocket = helpers::rocket::factory(pool);
        let client = Client::new(rocket).unwrap();
        let user = UserModel::create(&"test@test.com", &"lol", conn).expect("Error saving user");
    }

    it "should let you login" {
        let data = json!({"email": "test@test.com", "password": "lol"});
        println!("{}", data.to_string());
        let mut res = client.post("/api/auth/login")
                .header(ContentType::JSON)
                .body(data.to_string())
                .dispatch();
        assert_eq!(res.status(), Status::Ok);

        let data: Value = serde_json::from_str(&res.body_string().expect("no body string")).expect("JSON failed");
        let secret = &data.get("id").expect("no auth token").as_str().expect("not valid utf8");
        let mut res2 = client.get("/api/auth/whoami")
                .header(ContentType::JSON)
                .header(Header::new("Authorization", secret.to_string()))
                .dispatch();
        assert_eq!(res2.status(), Status::Ok);
    }
}
