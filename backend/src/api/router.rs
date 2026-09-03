use crate::api::{
    routes,
    state::{AppState, RouterDependencies},
};
use axum::Router;
pub fn router(dependencies: RouterDependencies) -> Router {
    routes::public::routes()
        .merge(routes::administration::routes())
        .merge(routes::reporting::routes())
        .merge(routes::settlement::routes())
        .with_state(AppState::from(dependencies))
}
