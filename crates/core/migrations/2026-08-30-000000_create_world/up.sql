CREATE TABLE inventories (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL
);

CREATE TABLE players (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    inventory_id INTEGER NOT NULL,
    CONSTRAINT players_inventory_id_fkey FOREIGN KEY (inventory_id) REFERENCES inventories(id) ON DELETE CASCADE
);

CREATE TABLE rooms (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    inventory_id INTEGER NOT NULL,
    CONSTRAINT rooms_inventory_id_fkey FOREIGN KEY (inventory_id) REFERENCES inventories(id) ON DELETE CASCADE
);

CREATE TABLE world_states (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    player_id INTEGER NOT NULL,
    current_room_id INTEGER NOT NULL,
    CONSTRAINT world_states_player_id_fkey FOREIGN KEY (player_id) REFERENCES players(id),
    CONSTRAINT world_states_current_room_id_fkey FOREIGN KEY (current_room_id) REFERENCES rooms(id)
);

CREATE TABLE items (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    primary_name TEXT NOT NULL,
    aliases TEXT NOT NULL DEFAULT ''
);

CREATE TABLE inventory_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    inventory_id INTEGER NOT NULL,
    item_id INTEGER NOT NULL,
    hidden BOOLEAN NOT NULL DEFAULT FALSE,
    CONSTRAINT inventory_items_inventory_id_fkey FOREIGN KEY (inventory_id) REFERENCES inventories(id) ON DELETE CASCADE,
    CONSTRAINT inventory_items_item_id_fkey FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE,
    CONSTRAINT inventory_items_inventory_id_item_id_unique UNIQUE (inventory_id, item_id)
);