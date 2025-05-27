use std::collections::HashMap;
use std::env;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use bloomfilter::Bloom;
use clap::Parser;
use crossbeam::sync::ShardedLock;
use mysql_async::prelude::*;
use sonyflake::Sonyflake;
use warp::http::Uri;
use warp::Filter;

use shorten_url::settings::load_settings;

fn short_url(short_code: &str) -> String {
    let default_domain = "http://localhost:3030/";
    let base_domain = env::var("DOMAIN").unwrap_or(default_domain.to_string());
    format!("{}{}", base_domain, short_code)
}
fn rendering_short_url_html(short_url: &str) -> String {
    format!(
        r#"
        <style>
            body {{
                font-family: Arial, sans-serif;
                margin: 40px;
                text-align: center;
            }}
            p {{
                background-color: #f4f4f4;
                border: 1px solid #e0e0e0;
                border-radius: 5px;
                display: inline-block;
                margin-top: 20px;
                padding: 10px 20px;
            }}
            a {{
                color: #3498db;
                text-decoration: none;
            }}
            a:hover {{
                text-decoration: underline;
            }}
        </style>
        <h2>Your Short URL</h2>
        <p>Short URL: <a href="{short_url}">{short_url}</a></p>
        "#,
        short_url = short_url,
    )
}

async fn generate_short_code(
    conn: &mut mysql_async::Conn,
    bloom: Arc<ShardedLock<Bloom<String>>>,
    original_url: String,
) -> Result<String> {
    let flake = Sonyflake::new().unwrap();
    let id = flake.next_id().unwrap();
    let short_code = base62::encode(id);
    println!("id: {}, Generated short URL: {}", id, short_code);

    conn.exec_drop(
        r"insert into url_mapping (id, short_code, original_url) values (:id, :short_code, :original_url)",
        params! {
            "id" => &id,
            "short_code" => &short_code,
            "original_url" => &original_url,
        },
    )
    .await
    .expect("Failed to insert data.");
    bloom.write().unwrap().set(&original_url);

    let short_url = short_url(&short_code);
    Ok(short_url)
}

async fn get_short_code(conn: &mut mysql_async::Conn, original_url: String) -> Result<String> {
    let short_codes = conn
        .exec_map(
            r"select short_code from url_mapping where original_url = :original_url",
            params! {
                "original_url" => &original_url,
            },
            |short_code: String| short_code,
        )
        .await
        .expect("Failed to select query.");

    if !short_codes.is_empty() {
        let short_url = short_url(&short_codes[0]);
        return Ok(short_url);
    }

    Err(anyhow::anyhow!("Not found short code"))
}

async fn shorten(
    form_data: HashMap<String, String>,
    pool: Arc<mysql_async::Pool>,
    bloom: Arc<ShardedLock<Bloom<String>>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut conn = pool
        .get_conn()
        .await
        .expect("Failed to connect to database.");

    let original_url = form_data
        .get("url")
        .unwrap_or(&String::from(""))
        .to_string();
    if !bloom.read().unwrap().check(&original_url) {
        let short_url = generate_short_code(&mut conn, bloom.clone(), original_url.clone())
            .await
            .unwrap();
        return Ok(warp::reply::html(rendering_short_url_html(&short_url)));
    }

    let short_url = get_short_code(&mut conn, original_url.clone()).await;
    if let Ok(short_url) = short_url {
        return Ok(warp::reply::html(rendering_short_url_html(&short_url)));
    };

    let short_url = generate_short_code(&mut conn, bloom.clone(), original_url.clone())
        .await
        .unwrap();
    Ok(warp::reply::html(rendering_short_url_html(&short_url)))
}

async fn redirect(
    short_code: String,
    pool: Arc<mysql_async::Pool>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut conn = pool
        .get_conn()
        .await
        .expect("Failed to connect to database.");

    let urls = conn
        .exec_map(
            r"select original_url from url_mapping where short_code = :short_code",
            params! {
                "short_code" => &short_code,
            },
            |original_url: String| original_url,
        )
        .await
        .expect("Failed to select query.");

    if !urls.is_empty() {
        println!("Found original URL: {}", urls[0]);
        let target_uri = Uri::from_str(&urls[0]).unwrap();
        println!("Redirect to: {}", target_uri);
        return Ok(warp::redirect(target_uri));
    }

    println!("Not found original URL");
    Err(warp::reject::not_found())
}

fn with<T: Clone + Send>(
    t: T,
) -> impl Filter<Extract = (T,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || t.clone())
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "settings.yaml")]
    settings: String,
}

async fn init_bloomfilter(pool: Arc<mysql_async::Pool>) -> Result<bloomfilter::Bloom<String>> {
    let num_items = 100000;
    let fp_rate = 0.001;
    let mut bloom = Bloom::new_for_fp_rate(num_items, fp_rate).unwrap();
    let mut conn = pool
        .get_conn()
        .await
        .expect("Failed to connect to database.");
    let urls = conn
        .exec_map(
            r"select original_url from url_mapping",
            (),
            |original_url: String| original_url,
        )
        .await
        .expect("Failed to select query");
    for url in urls {
        bloom.set(&url);
    }
    Ok(bloom)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let settings = load_settings(args.settings).expect("failed to load settings from yaml");

    let database_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        settings.mysql.user,
        settings.mysql.password,
        settings.mysql.host,
        settings.mysql.port,
        settings.mysql.db
    );
    let database_url = mysql_async::Opts::from_str(&database_url).unwrap();
    let pool = mysql_async::Pool::new(database_url);
    let pool = Arc::new(pool);
    let index = warp::path::end().map(|| {
        warp::reply::html(
            r#"
            <style>
                body { font-family: Arial, sans-serif; margin: 40px; text-align: center; }
                input[type="text"] { padding: 10px; width: 300px; }
                input[type="submit"] { padding: 10px 20px; }
            </style>
            <h2>URL Shortener</h2>
            <form action="/shorten" method="post">
                <input type="text" name="url" placeholder="Enter your URL here">
                <br><br>
                <input type="submit" value="Shorten">
            </form>
            "#,
        )
    });
    let bloom = init_bloomfilter(pool.clone()).await.unwrap();
    let bloom = Arc::new(ShardedLock::new(bloom));

    let shorten_route = warp::path!("shorten")
        .and(warp::post())
        .and(warp::body::form())
        .and(with(pool.clone()))
        .and(with(bloom.clone()))
        .and_then(shorten);
    let redirect_route = warp::path!(String)
        .and(with(pool.clone()))
        .and_then(redirect);
    let routes = index.or(shorten_route).or(redirect_route);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
