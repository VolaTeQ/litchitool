use litchitool::mission::Coordinate;
use serde::Deserialize;
use serde_json::Value;

use crate::error::LitchiApiError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectId(pub(crate) String);

#[derive(Debug, Deserialize)]
pub struct SessionData {
    #[serde(rename = "objectId")]
    pub object_id: String,
    pub username: String,
    pub email: String,
    pub name: String,
    #[serde(rename = "emailVerified")]
    pub email_verified: bool,
    #[serde(rename = "sessionToken")]
    pub(crate) session_token: String,
}

#[derive(Debug, Clone)]
pub struct MissionFile {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Mission {
    pub object_id: ObjectId,
    pub location: Coordinate,
    pub name: String,
    pub user_id: ObjectId,
    pub file: MissionFile,
}

impl TryFrom<&Value> for Mission {
    type Error = LitchiApiError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let format_error = |error: &str| LitchiApiError::MissionFormatError(error.to_string());

        let object_id = ObjectId(
            value
                .get("objectId")
                .and_then(|value| value.as_str())
                .ok_or(format_error("Could not get objectId of mission"))?
                .to_string(),
        );
        let location = value
            .get("location")
            .and_then(|location_map| {
                Some(Coordinate(
                    location_map.get("latitude")?.as_f64()?,
                    location_map.get("longitude")?.as_f64()?,
                ))
            })
            .ok_or(format_error("Could not get location of mission"))?;
        let name = value
            .get("name")
            .and_then(|mission| mission.as_str())
            .ok_or(format_error("Could not get name of mission"))?
            .to_string();
        let user_id = ObjectId(
            value
                .get("user")
                .and_then(|user| user.get("objectId")?.as_str())
                .ok_or(format_error("Could not get user of mission"))?
                .to_string(),
        );
        let file = value
            .get("file")
            .and_then(|file_map| {
                Some(MissionFile {
                    name: file_map.get("name")?.as_str()?.to_string(),
                    url: file_map.get("url")?.as_str()?.to_string(),
                })
            })
            .ok_or(format_error("Could not get file of mission"))?;

        Ok(Mission {
            object_id,
            location,
            name,
            user_id,
            file,
        })
    }
}
