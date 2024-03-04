pub mod error;
mod types;

pub use types::*;

use error::LitchiApiError;
use litchitool::mission::LitchiMission;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Response,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{debug, instrument, trace};

const APP_ID: &str = "APjd97yuFQ9TUiIIKgDiqzczon1z2339RxINQe6g";
static API_BASE: &str = "https://parse.litchiapi.com";

pub struct LitchiApi {
    client: Client,
    session_data: SessionData,
}

impl LitchiApi {
    pub async fn login(username: &str, password: &str) -> Result<Self, LitchiApiError> {
        let mut client = Client::builder()
            .default_headers(HeaderMap::from_iter([(
                HeaderName::from_static("x-parse-application-id"),
                HeaderValue::from_static(APP_ID),
            )]))
            .cookie_store(true)
            .build()?;

        let session_data = Self::authenticate(&mut client, username, password).await?;

        Ok(Self {
            client,
            session_data,
        })
    }

    #[instrument(skip(password, client), err)]
    async fn authenticate(
        client: &mut Client,
        username: &str,
        password: &str,
    ) -> Result<SessionData, LitchiApiError> {
        let url = API_BASE.to_string() + "/parse/login";

        #[derive(Serialize)]
        struct LoginPayload<'a> {
            username: &'a str,
            password: &'a str,
        }

        let result = client
            .post(url)
            .json(&LoginPayload { username, password })
            .send()
            .await?;

        if !result.status().is_success() {
            Err(LitchiApiError::AuthError(result.text().await?))
        } else {
            Ok(result.json().await?)
        }
    }

    pub fn user_data(&self) -> &SessionData {
        &self.session_data
    }

    #[instrument(skip_all, fields(mission_name = %name), err)]
    pub async fn upload(
        &self,
        mission: &LitchiMission,
        name: &str,
    ) -> Result<ObjectId, LitchiApiError> {
        let url = API_BASE.to_string() + "/parse/files/mission";

        #[derive(Deserialize)]
        struct UploadResult {
            name: String,
            url: String,
        }

        trace!("Converting mission to binary");
        let mission_bin = mission.to_binary();
        debug!("Uploading mission binary blob to litchi");
        let mission_file = self
            .client
            .post(url)
            .header("Content-Type", "application/octet-stream")
            .header("X-Parse-Session-Token", &self.session_data.session_token)
            .body(mission_bin)
            .send()
            .await?;
        trace!("Parsing mission data upload result");
        let mission_file: UploadResult = mission_file.json().await?;

        let (mission_lat, mission_long) =
            mission.waypoints().get(0).map_or((0.0, 0.0), |waypoint| {
                (waypoint.coordinate.0, waypoint.coordinate.1)
            });

        let upload_payload = json!({
            "ACL": {
                &self.session_data.object_id: {
                    "read": true,
                    "write": true,
                }
            },
            "location": {
                "__type": "GeoPoint",
                "latitude": mission_lat,
                "longitude": mission_long,
            },
            "name": name,
            "user": {
                "__type": "Pointer",
                "className": "_User",
                "objectId": self.session_data.object_id,
            },
            "file": {
                "__type": "File",
                "name": mission_file.name,
                "url": mission_file.url,
            }
        });

        debug!("Creating mission object");
        let create_mission_response: serde_json::Value = check_api_response(
            self.client
                .post(API_BASE.to_string() + "/parse/classes/Mission")
                .header("X-Parse-Session-Token", &self.session_data.session_token)
                .json(&upload_payload)
                .send()
                .await?,
        )
        .await?
        .json()
        .await?;

        trace!("Parsing mission object creation response");
        create_mission_response
            .get("objectId")
            .and_then(|object_id| Some(ObjectId(object_id.as_str()?.to_string())))
            .ok_or(LitchiApiError::ResponseFormateError(
                "Response has no objectId".to_string(),
                create_mission_response.to_string(),
            ))
    }

    #[instrument(skip(self), err)]
    pub async fn missions(&self) -> Result<Vec<Mission>, LitchiApiError> {
        let payload = json!({
           "where": {
                "user": {
                    "__type": "Pointer",
                    "className": "_User",
                    "objectId": self.session_data.object_id,
                }
            }
        });
        debug!("Requesting misssions");
        let response = self
            .client
            .get(API_BASE.to_string() + "/parse/classes/Mission")
            .header("X-Parse-Session-Token", &self.session_data.session_token)
            .json(&payload)
            .send()
            .await?;

        // Check status
        let response: serde_json::Value = check_api_response(response).await?.json().await?;

        response
            .get("results")
            .and_then(|results| results.as_array())
            .ok_or(LitchiApiError::ResponseFormateError(
                "response should have results array field".to_string(),
                response.to_string(),
            ))
            .and_then(|results| {
                results
                    .iter()
                    .map(|result| result.try_into())
                    .collect::<Result<_, _>>()
            })
    }

    #[instrument(skip(self), err)]
    pub async fn delete_mission(&self, mission_id: ObjectId) -> Result<(), LitchiApiError> {
        debug!("Requesting to delete mission");
        let response = self
            .client
            .delete(format!(
                "{}/parse/classes/Mission/{}",
                API_BASE, mission_id.0
            ))
            .header("X-Parse-Session-Token", &self.session_data.session_token)
            .send()
            .await?;

        check_api_response(response).await?;

        Ok(())
    }

    #[instrument(skip(self), err)]
    pub async fn sync_devices(&self) -> Result<(), LitchiApiError> {
        debug!("Synchronizing devices");
        let response = self
            .client
            .post(API_BASE.to_string() + "/parse/functions/syncMyDevices")
            .header("X-Parse-Session-Token", &self.session_data.session_token)
            .send()
            .await?;

        check_api_response(response).await?;

        Ok(())
    }
}

async fn check_api_response(response: Response) -> Result<Response, LitchiApiError> {
    if !response.status().is_success() {
        Err(LitchiApiError::HTTPError(
            response.status().as_u16(),
            response.text().await?,
        ))
    } else {
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use crate::{error::LitchiApiError, LitchiApi};

    #[tokio::test]
    async fn test_upload_mission() -> Result<(), LitchiApiError> {
        const MISSION_DATA: &[u8] = include_bytes!("../../litchitool/test/litchi_mission.csv");

        let secret_username =
            std::env::var("LITCHI_USERNAME").expect("Must have $LITCHI_USERNAME set for tests");
        let secret_password =
            std::env::var("LITCHI_PASSWORD").expect("Must have $LITCHI_PASSWORD set for tests");

        let mission = litchitool::csv_format::read_from_csv(csv::Reader::from_reader(MISSION_DATA))
            .expect("Could not parse csv mission");

        let api = LitchiApi::login(&secret_username, &secret_password).await?;

        // Upload a test mission
        let uploaded = api.upload(&mission, "testingmisssion").await?;

        // Sync devices just in case
        api.sync_devices().await?;

        // Check if that mission actually was uploaded
        let current_missions = api.missions().await?;
        assert!(current_missions
            .iter()
            .any(|mission| mission.object_id == uploaded));

        // Clear that mission again
        api.delete_mission(uploaded).await?;

        Ok(())
    }
}
