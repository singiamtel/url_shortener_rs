// @generated automatically by Diesel CLI.

diesel::table! {
    url (id) {
        id -> Int4,
        name -> Text,
        short_url -> Text,
        created_at -> Nullable<Timestamp>,
        created_by -> Text,
    }
}
