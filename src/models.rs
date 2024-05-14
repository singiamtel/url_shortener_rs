use crate::schema::url;
use chrono::NaiveDateTime;
use diesel::prelude::*;

#[derive(Queryable, Selectable)]
#[diesel(table_name = url)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Url {
    pub id: i32,
    pub name: String,
    pub short_url: String,
    pub created_at: Option<NaiveDateTime>,
    pub created_by: String,
}

#[derive(Insertable)]
#[diesel(table_name = url)]
pub struct NewUrl<'a> {
    pub name: &'a str,
    pub short_url: &'a str,
    pub created_by: &'a str,
}
