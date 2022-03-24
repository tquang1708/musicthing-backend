## WIP senior project - Backend

### Installation

This project was built on Postgres 13.6 and Rust 1.59.0. I might add an option to install from Docker for comfort but I can't get it to work on my machine.

In the meantime, **simply executing `run.sh`** should install the database as well as everything on your own system while also launching the backend itself.

### Class Note

`src/handlers/list.rs` and `src/handlers/reload.rs` are the 2 files that have the most work put into them, as these 2 implement the "list" and the "reload" endpoints on the api. It would be good if this can be given a look at.
