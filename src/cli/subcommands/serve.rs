use clap::{App, ArgMatches};
use feedspool::db;
use feedspool::gql::{mutation::RootMutation, query::RootQuery, Context};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Response, Server, StatusCode,
};
use hyper_staticfile::Static;
use juniper::{EmptySubscription, RootNode};
use std::error::Error;
use std::net::SocketAddr;
use std::path::Path;
use std::{convert::Infallible, sync::Arc};

pub const NAME: &str = "serve";

pub fn app() -> App<'static> {
    App::new(NAME).about("Start web API server")
}

pub async fn execute(_matches: &ArgMatches, config: &config::Config) -> Result<(), Box<dyn Error>> {
    let db_pool = db::create_pool(config)?;

    let root_node = Arc::new(RootNode::new(
        RootQuery,
        RootMutation,
        EmptySubscription::<Context>::new(),
    ));

    let ctx = Arc::new(Context { pool: db_pool });

    let staticfiles = Static::new(Path::new(&config.get::<String>("http_server_static_path")?));

    // TODO: move all this http server stuff into its own module outside of subcommands?
    let new_service = make_service_fn(move |_| {
        let root_node = root_node.clone();
        let ctx = ctx.clone();
        let staticfiles = staticfiles.clone();

        // TODO: break down this indentation pyramid into separate functions?
        async {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                let root_node = root_node.clone();
                let ctx = ctx.clone();
                let staticfiles = staticfiles.clone();
                async move {
                    Ok::<_, Infallible>(match (req.method(), req.uri().path()) {
                        (&Method::GET, "/graphiql") => {
                            juniper_hyper::graphiql("/graphql", None).await
                        }
                        (&Method::GET, "/graphql") | (&Method::POST, "/graphql") => {
                            juniper_hyper::graphql(root_node, ctx, req).await
                        }
                        _ => match staticfiles.serve(req).await {
                            Ok(resp) => resp,
                            Err(err) => {
                                let mut response = Response::new(Body::from(format!("{:?}", err)));
                                *response.status_mut() = StatusCode::NOT_FOUND;
                                response
                            }
                        },
                    })
                }
            }))
        }
    });

    let server_addr: SocketAddr = config.get::<String>("http_server_address")?.parse()?;
    let server = Server::bind(&server_addr).serve(new_service);
    println!("Listening on http://{}", server_addr);

    if let Err(e) = server.await {
        eprintln!("server error: {}", e)
    }

    Ok(())
}
