USE shorten;

CREATE TABLE IF NOT EXISTS url_mapping (
    id BIGINT PRIMARY KEY,
    short_code VARCHAR(255) NOT NULL,
    original_url VARCHAR(1024) NOT NULL
);

GRANT ALL PRIVILEGES ON shorten.* TO 'user'@'%';
FLUSH PRIVILEGES;
