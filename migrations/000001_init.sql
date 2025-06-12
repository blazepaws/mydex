create database if not exists `mydex`;
use `mydex`;

# Primary user table. Contains basic information about a user.
create table if not exists `user` (
    `user_id` integer primary key auto_increment,
    `name` varchar(64) unique not null,
    `creation_date` timestamp default current_timestamp not null,
    `password` varchar(1024) not null
);

# Associate groups with users
create table if not exists `user_group` (
    `user_id` integer not null,
    `group` varchar(128) not null
);

# Assigns each group a set of permissions.
create table if not exists `group_permission` (
    `group` varchar(128) not null,
    `permission` varchar(128) not null
);

# Pokedex definition
create table if not exists `pokedex` (
    # URL-safe short ID for identification
    `id` varchar(256) primary key not null,
    # The name we show to the user
    `name` varchar(256) unique not null,
    # A description what this pokedex is about
    `description` text not null,
    `num_entries` integer not null,
    `thumbnail_url` varchar(256) not null,
    `spritesheet_url` varchar(256) not null,
    # The commit hash of the last update to the data.
    # Used to determine if the data is outdated.
    `commit_hash` varchar(64) not null,
    # The actual definition of the pokedex entries.
    # This is json for simplicity. It is only loaded and written in whole.
    `entries` json not null
);

# Defines which dexes a user has added to their profile
create table if not exists `user_pokedex_progress` (
    `user_id` integer not null,
    `pokedex_id` varchar(256) not null,
    # Each collected pokemon's entry_id in a dex is stored.
    # If an ID is in this table, the pokemon counts as collected.
    `entry_id` integer not null,
    # Each row represents a unique collected pokemon
    primary key (`user_id`, `pokedex_id`, `entry_id`),
    # The dex_name field references the name property of the pokedex table.
    foreign key (`pokedex_id`) references `pokedex` (`id`),
    # We often ask which entries a user has in a specific pokedex,
    # so optimize for that case.
    index `pokedex_progress` (`user_id`, `pokedex_id`)
);

create table if not exists `user_pokedex` (
   `user_id` integer not null,
   `pokedex_id` varchar(256) not null,
   foreign key (`pokedex_id`) references `pokedex` (`id`)
);

start transaction;
# Create a default 'admin' user with the initial password 'admin'
insert into `user` (`user_id`, `name`, `password`) values (null, 'admin', '$argon2id$v=19$m=19456,t=2,p=1$N+BDJucGFJyl8tGGMNT8BQ$TU+fG7P52u+kyxHt32QswENR+yBhc+lVyCjFWYGUguc');
# Add them to the admin group
insert into `user_group` values ((select `user_id` from `user` where `name` = 'admin'), 'admin');
commit;

