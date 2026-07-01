//! Shared current-user resource for admin-only public-page affordances.

use crate::auth::session::SessionUser;
use leptos::prelude::*;
use leptos::server_fn::ServerFnError;

pub type CurrentUserResult = Result<Option<SessionUser>, ServerFnError>;

#[derive(Clone)]
pub struct CurrentUserResource(pub ArcOnceResource<CurrentUserResult>);

pub fn provide_current_user_resource(resource: ArcOnceResource<CurrentUserResult>) {
    provide_context(CurrentUserResource(resource));
}

pub fn use_current_user_resource() -> Option<ArcOnceResource<CurrentUserResult>> {
    use_context::<CurrentUserResource>().map(|resource| resource.0)
}

pub fn user_is_admin(user_res: &CurrentUserResult) -> bool {
    matches!(user_res, Ok(Some(session)) if session.is_admin)
}

/// Wrapper that renders `children` only when the current user is an admin.
///
/// If no current-user resource is in context, renders nothing.
/// During Suspense pending, renders nothing.
#[component]
pub fn AdminOnly(children: ChildrenFn) -> impl IntoView {
    let Some(user) = use_current_user_resource() else {
        return ().into_any();
    };

    view! {
        <Suspense fallback=|| ()>
            <Show when=move || {
                user.get().map(|res| user_is_admin(&res)).unwrap_or(false)
            }>
                {children()}
            </Show>
        </Suspense>
    }
    .into_any()
}
