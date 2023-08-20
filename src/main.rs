use warp::Filter;
use mysql_async::prelude::*;

async fn shorten(body: warp::hyper::body::Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    let input_url = String::from_utf8(body.to_vec()).unwrap_or_default();
    println!("Received URL: {}", input_url);

    let short_url = save_to_mysql(&input_url).await;

    Ok(warp::reply::html(format!(
        r#"
        <p>Shortened URL: <a href="{short_url}">{short_url}</a></p>
        "#,
        short_url = short_url,
    )))
}

async fn save_to_mysql(url: &str) -> String {
    let database_url = mysql_async::Opts::from_url("mysql://user:password@127.0.0.1:3306/shorten").unwrap();
    let pool = mysql_async::Pool::new(database_url);
    let mut conn = pool.get_conn().await.expect("Failed to connect to database.");

    let short_url = "http://short.url/abcd1234".to_string(); // dummy url

    conn.exec_drop(
        r"INSERT INTO url_mapping (short_url, original_url) VALUES (:short_url, :original_url)",
        params! {
            "short_url" => &short_url,
            "original_url" => &url,
        },
    )
    .await
    .expect("Failed to insert data.");

    drop(conn);

    short_url
}

#[tokio::main]
async fn main() {
    let index = warp::path::end()
        .map(|| warp::reply::html(
            r#"
            <form action="/shorten" method="post">
                <input type="text" name="url">
                <input type="submit" value="Shorten">
            </form>
            "#
        ));

    let shorten_route = warp::path!("shorten")
        .and(warp::post())
        .and(warp::body::bytes())
        .and_then(shorten);

    let routes = index.or(shorten_route);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}
