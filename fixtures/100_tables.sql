CREATE TABLE IF NOT EXISTS `shorten`.`url_mapping` (
    id INT AUTO_INCREMENT PRIMARY KEY,
    short_url VARCHAR(255) NOT NULL,
    original_url LONGTEXT NOT NULL,
);
