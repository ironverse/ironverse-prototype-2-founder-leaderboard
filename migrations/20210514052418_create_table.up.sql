CREATE TABLE IF NOT EXISTS founder_answers
(
    id          BIGSERIAL PRIMARY KEY,
    email       TEXT    NOT NULL,
    username    TEXT    NOT NULL,
    question    TEXT    NOT NULL,
    answer      TEXT    NOT NULL,
    created_on  timestamp without time zone NOT NULL default (now() at time zone 'utc'),
    updated_on  timestamp without time zone NOT NULL default (now() at time zone 'utc'),
    UNIQUE (email, question)
);