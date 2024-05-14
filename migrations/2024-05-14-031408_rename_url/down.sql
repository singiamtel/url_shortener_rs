-- This file should undo anything in `up.sql`

alter table URL rename column name to url;
