// Copyright 2025 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use actix_web::{Error, dev::ServiceRequest};
#[cfg(feature = "enterprise")]
use actix_web::{
    error::ErrorUnauthorized,
    http::{Method, header},
};
#[cfg(feature = "enterprise")]
use o2_dex::{config::get_config as get_dex_config, service::auth::get_dex_jwks};

use crate::common::utils::auth::AuthExtractor;
#[cfg(feature = "enterprise")]
use crate::common::utils::jwt;
#[cfg(feature = "enterprise")]
use crate::service::{db, users};

#[cfg(feature = "enterprise")]
pub async fn token_validator(
    req: ServiceRequest,
    auth_info: AuthExtractor,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    use actix_web::error::ErrorForbidden;

    use super::validator::check_permissions;
    use crate::common::utils::auth::V2_API_PREFIX;

    let user;
    let keys = get_dex_jwks().await;
    let path = match req
        .request()
        .path()
        .strip_prefix(format!("{}/api/", config::get_config().common.base_uri).as_str())
    {
        Some(path) => path,
        None => req.request().path(),
    };
    let path_columns = path.split('/').collect::<Vec<&str>>();

    match jwt::verify_decode_token(
        auth_info.auth.strip_prefix("Bearer").unwrap().trim(),
        &keys,
        &get_dex_config().client_id,
        false,
        true,
    ) {
        Ok(res) => {
            let user_id = &res.0.user_email;
            if res.0.is_valid {
                // for member sub i.e. invitation, we must check user directly from db, because
                // the else-arm here will check if user is present in given org. However before
                // accepting the invitation, user will not be added to the org,
                // hence getting unauthorized error. Thus we add a check that for
                // member_subscription, check the user directly from db users,
                // not particularly associated with any org
                let is_member_subscription = path_columns
                    .get(1)
                    .is_some_and(|p| p.eq(&"member_subscription"));
                let path_suffix = path_columns.last().unwrap_or(&"");
                if path_suffix.eq(&"organizations")
                    || path_suffix.eq(&"clusters")
                    || is_member_subscription
                {
                    let db_user = db::user::get_db_user(user_id).await;
                    user = match db_user {
                        Ok(user) => {
                            let all_users = user.get_all_users();
                            if all_users.is_empty() {
                                None
                            } else {
                                all_users.first().cloned()
                            }
                        }
                        Err(e) => {
                            log::error!("Error getting user in token validator: {e}");
                            None
                        }
                    }
                } else {
                    user = match path.find('/') {
                        Some(index) => {
                            let org_id =
                                if path_columns.len() > 1 && path_columns[0].eq(V2_API_PREFIX) {
                                    path_columns[1]
                                } else {
                                    &path[0..index]
                                };
                            users::get_user(Some(org_id), user_id).await
                        }
                        None => users::get_user(None, user_id).await,
                    }
                };
                match user {
                    Some(user) => {
                        // / Hack for prometheus, need support POST and check the header
                        let mut req = req;

                        if req.method().eq(&Method::POST)
                            && !req.headers().contains_key("content-type")
                        {
                            req.headers_mut().insert(
                                header::CONTENT_TYPE,
                                header::HeaderValue::from_static(
                                    "application/x-www-form-urlencoded",
                                ),
                            );
                        }
                        req.headers_mut().insert(
                            header::HeaderName::from_static("user_id"),
                            header::HeaderValue::from_str(&res.0.user_email).unwrap(),
                        );
                        if auth_info.bypass_check
                            || check_permissions(user_id, auth_info, user.role, user.is_external)
                                .await
                        {
                            Ok(req)
                        } else {
                            Err((ErrorForbidden("Unauthorized Access"), req))
                        }
                    }
                    _ => Err((ErrorForbidden("Unauthorized Access"), req)),
                }
            } else {
                Err((ErrorForbidden("Unauthorized Access"), req))
            }
        }
        Err(err) => Err((ErrorUnauthorized(err), req)),
    }
}

#[cfg(feature = "enterprise")]
pub async fn get_user_name_from_token(auth_str: &str) -> Option<String> {
    let keys = get_dex_jwks().await;
    match jwt::verify_decode_token(
        auth_str.strip_prefix("Bearer").unwrap().trim(),
        &keys,
        &get_dex_config().client_id,
        false,
        true,
    ) {
        Ok(res) => {
            if res.0.is_valid {
                Some(res.0.user_email)
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

#[cfg(not(feature = "enterprise"))]
pub async fn token_validator(
    req: ServiceRequest,
    _token: AuthExtractor,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    use actix_web::error::ErrorForbidden;

    Err((ErrorForbidden("Not Supported"), req))
}
