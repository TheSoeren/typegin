// @generated automatically by Diesel CLI.

diesel::table! {
    inventories (id) {
        id -> Integer,
    }
}

diesel::table! {
    inventory_items (id) {
        id -> Integer,
        inventory_id -> Integer,
        item_id -> Integer,
        hidden -> Bool,
    }
}

diesel::table! {
    items (id) {
        id -> Integer,
        primary_name -> Text,
        aliases -> Text,
    }
}

diesel::table! {
    players (id) {
        id -> Integer,
        inventory_id -> Integer,
    }
}

diesel::table! {
    rooms (id) {
        id -> Integer,
        inventory_id -> Integer,
    }
}

diesel::table! {
    world_states (id) {
        id -> Integer,
        player_id -> Integer,
        current_room_id -> Integer,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    inventories,
    inventory_items,
    items,
    players,
    rooms,
    world_states,
);