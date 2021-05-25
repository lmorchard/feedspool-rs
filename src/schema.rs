table! {
    entries (id) {
        id -> Nullable<Text>,
        feed_id -> Nullable<Text>,
        published -> Nullable<Text>,
        created_at -> Nullable<Text>,
        modified_at -> Nullable<Text>,
        defunct -> Nullable<Bool>,
        json -> Nullable<Text>,
        guid -> Nullable<Text>,
        title -> Nullable<Text>,
        link -> Nullable<Text>,
        summary -> Nullable<Text>,
        content -> Nullable<Text>,
        updated -> Nullable<Text>,
    }
}

table! {
    feed_history (id) {
        id -> Nullable<Text>,
        feed_id -> Nullable<Text>,
        created_at -> Nullable<Text>,
        updated_at -> Nullable<Text>,
        src -> Nullable<Text>,
        status -> Nullable<Text>,
        etag -> Nullable<Text>,
        last_modified -> Nullable<Text>,
        json -> Nullable<Text>,
        is_error -> Nullable<Bool>,
        error_text -> Nullable<Text>,
    }
}

table! {
    feeds (id) {
        id -> Nullable<Text>,
        published -> Nullable<Text>,
        created_at -> Nullable<Text>,
        modified_at -> Nullable<Text>,
        url -> Nullable<Text>,
        title -> Nullable<Text>,
        subtitle -> Nullable<Text>,
        link -> Nullable<Text>,
        json -> Nullable<Text>,
        updated -> Nullable<Text>,
        last_entry_published -> Nullable<Text>,
    }
}

allow_tables_to_appear_in_same_query!(entries, feed_history, feeds,);
