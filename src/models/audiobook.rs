use helpers::uuid::Uuid;
use diesel;
use diesel::prelude::*;
use schema::audiobooks;
use schema::playstates;
use schema::library_permissions;
use std::path::Path;
use diesel::result::Error;
use ::models::library::Library;
use ::models::chapter::Chapter;
use chrono::NaiveDateTime;
use diesel::sqlite::SqliteConnection;
use models::user::User;
use models::permission::Permission;

#[table_name="audiobooks"]
#[derive(PartialEq, Debug, Queryable, AsChangeset, Associations, Identifiable, Serialize, Clone,
         Insertable)]
#[hasmany(chapters)]
#[belongs_to(Library)]
pub struct Audiobook {
    pub id: Uuid,
    pub location: String,
    pub title: String,
    pub artist: Option<String>,
    pub length: f64,
    pub library_id: Uuid,
    pub hash: Vec<u8>,
    pub file_extension: String,
    pub deleted: bool
}

pub enum Update {
    Nothing,
    Path,
    NotFound
}

impl Audiobook {
    fn find_by_hash(hash: &[u8], conn: &diesel::sqlite::SqliteConnection) -> Result<Audiobook, diesel::result::Error> {
        audiobooks::dsl::audiobooks.filter(audiobooks::dsl::hash.eq(hash)).get_result(conn)
    }

    /// Updates the path of any book with the given hash to the new_path provided.
    /// Returns true if a path is now correct, returns false if no book with this hash exists.
    pub fn update_path(book_hash: &[u8], new_path: &AsRef<str>, conn: &diesel::sqlite::SqliteConnection)
        -> Result<Update, diesel::result::Error> {
        if let Ok(book) = Self::find_by_hash(book_hash, conn) {
            if book.location != new_path.as_ref() {
                diesel::update(audiobooks::dsl::audiobooks.filter(audiobooks::dsl::hash.eq(book_hash)))
                    .set(audiobooks::dsl::location.eq(new_path.as_ref())).execute(conn)?;
                return Ok(Update::Path)
            };
            return Ok(Update::Nothing)
        } else {
            return Ok(Update::NotFound);
        }
    }

    pub fn delete_all_chapters(&self, conn: &diesel::sqlite::SqliteConnection) -> diesel::result::QueryResult<usize> {
        diesel::delete(Chapter::belonging_to(self)).execute(&*conn)
    }

    pub fn ensure_exists_in(relative_path: &AsRef<str>, library: &Library,
                            new_book: &Audiobook, conn: &SqliteConnection)
        -> Result<Audiobook, diesel::result::Error> {
        match Self::belonging_to(library)
            .filter(audiobooks::dsl::location.eq(relative_path.as_ref()))
            .first(&*conn)
            .optional()? {
                Some(b) => {
                    diesel::update(audiobooks::table).set(new_book).execute(conn)?;
                    Ok(b)
                },
                None => {
                    diesel::insert_into(audiobooks::table).values(new_book).get_result::<Audiobook>(conn)
                }
            }
    }
}
