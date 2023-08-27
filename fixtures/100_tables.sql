-- データベースへの移動
USE shorten;

-- url_mappingテーブルの作成
CREATE TABLE IF NOT EXISTS url_mapping (
    id BIGINT PRIMARY KEY,
    short_url VARCHAR(255) NOT NULL,
    original_url VARCHAR(1024) NOT NULL
);

-- userに対する権限の付与
GRANT ALL PRIVILEGES ON shorten.* TO 'user'@'%';
FLUSH PRIVILEGES;
