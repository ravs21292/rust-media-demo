// @generated automatically by Diesel CLI.

diesel::table! {
    media_files (id) {
        id -> Nullable<Integer>,
        name -> Text,
        path -> Text,
        uploaded_at -> Timestamp,
    }
}
