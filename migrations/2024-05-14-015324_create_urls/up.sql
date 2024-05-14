-- Your SQL goes here

create table URL (
    id serial primary key,
    url text not null,
    short_url text not null,
    created_at timestamp default current_timestamp,
    created_by text not null -- IP address
);
