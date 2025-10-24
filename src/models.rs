use diesel::prelude::*;
use chrono::NaiveDateTime;
use serde::{Serialize, Deserialize};

#[derive(Queryable, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::media_files)]
pub struct MediaFile {
    pub id: i32,
    pub name: String,
    pub path: String,
    pub uploaded_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::media_files)]
pub struct NewMediaFile {
    pub name: String,
    pub path: String,
}