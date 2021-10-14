use std::net::{Ipv4Addr, SocketAddr};
use warp::Filter;

macro_rules! static_file {
    ($file:expr) => {{
        let body = include_bytes!(concat!("../assets/", $file));
        let path = warp::path::end().or(warp::path("static")
            .and(warp::path($file))
            .and(warp::path::end()));
        let content_type = match $file {
            "index.html" => "text/html",
            "favicon.ico" => "image/x-icon",
            _ => unreachable!(),
        };
        let reply = move |_| warp::reply::with_header(&body[..], "content-type", content_type);
        warp::get().and(path).map(reply)
    }};
    ($dir:expr, $file:expr) => {{
        let body = include_bytes!(concat!("../assets/", $dir, "/", $file));
        let path = warp::path("static")
            .and(warp::path($dir))
            .and(warp::path($file))
            .and(warp::path::end());
        let content_type = match $dir {
            "css" => "text/css",
            "js" => "application/javascript",
            _ => unreachable!(),
        };
        let reply = move || warp::reply::with_header(&body[..], "content-type", content_type);
        warp::get().and(path).map(reply)
    }};
}

#[tokio::main]
async fn main() {
    let f1 = static_file!("index.html");
    let f2 = static_file!("favicon.ico");
    let f3 = static_file!("css", "app.css");
    let f4 = static_file!("css", "chunk-vendors.css");
    let f5 = static_file!("js", "app.js");
    let f6 = static_file!("js", "app.js.map");
    let f7 = static_file!("js", "chunk-vendors.js");
    let f8 = static_file!("js", "chunk-vendors.js.map");
    let filter = f1.or(f2).or(f3).or(f4).or(f5).or(f6).or(f7).or(f8);

    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 2222));
    println!("masshd listening on http://{}", addr);
    warp::serve(filter).run(addr).await;
}
