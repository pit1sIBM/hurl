/*
 * Hurl (https://hurl.dev)
 * Copyright (C) 2023 Orange
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */

use crate::http::easy_ext::CertInfo;
use chrono::{DateTime, NaiveDateTime, Utc};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Certificate {
    pub subject: String,
    pub issuer: String,
    pub start_date: DateTime<Utc>,
    pub expire_date: DateTime<Utc>,
    pub serial_number: String,
}

impl TryFrom<CertInfo> for Certificate {
    type Error = String;

    /// parse `cert_info`
    /// support different "formats" in cert info
    /// - attribute name: "Start date" vs "Start Date"
    /// - date format: "Jan 10 08:29:52 2023 GMT" vs "2023-01-10 08:29:52 GMT"
    fn try_from(cert_info: CertInfo) -> Result<Self, Self::Error> {
        let attributes = parse_attributes(&cert_info.data);
        let subject = parse_subject(&attributes)?;
        let issuer = parse_issuer(&attributes)?;
        let start_date = parse_start_date(&attributes)?;
        let expire_date = parse_expire_date(&attributes)?;
        let serial_number = parse_serial_number(&attributes)?;
        Ok(Certificate {
            subject,
            issuer,
            start_date,
            expire_date,
            serial_number,
        })
    }
}

fn parse_subject(attributes: &HashMap<String, String>) -> Result<String, String> {
    attributes
        .get("subject")
        .cloned()
        .ok_or(format!("missing Subject attribute in {attributes:?}"))
}

fn parse_issuer(attributes: &HashMap<String, String>) -> Result<String, String> {
    attributes
        .get("issuer")
        .cloned()
        .ok_or(format!("missing issuer attribute in {attributes:?}"))
}

fn parse_start_date(attributes: &HashMap<String, String>) -> Result<DateTime<Utc>, String> {
    match attributes.get("start date") {
        None => Err(format!("missing start date attribute in {attributes:?}")),
        Some(value) => Ok(parse_date(value)?),
    }
}

fn parse_expire_date(attributes: &HashMap<String, String>) -> Result<DateTime<Utc>, String> {
    match attributes.get("expire date") {
        None => Err("missing expire date attribute".to_string()),
        Some(value) => Ok(parse_date(value)?),
    }
}

fn parse_date(value: &str) -> Result<DateTime<Utc>, String> {
    let naive_date_time = match NaiveDateTime::parse_from_str(value, "%b %d %H:%M:%S %Y GMT") {
        Ok(d) => d,
        Err(_) => NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S GMT")
            .map_err(|_| format!("can not parse date <{value}>"))?,
    };
    Ok(naive_date_time.and_local_timezone(Utc).unwrap())
}

fn parse_serial_number(attributes: &HashMap<String, String>) -> Result<String, String> {
    attributes
        .get("serial number")
        .cloned()
        .ok_or(format!("Missing serial number attribute in {attributes:?}"))
}

fn parse_attributes(data: &Vec<String>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for s in data {
        if let Some((name, value)) = parse_attribute(s) {
            map.insert(name.to_lowercase(), value);
        }
    }
    map
}

fn parse_attribute(s: &str) -> Option<(String, String)> {
    if let Some(index) = s.find(':') {
        let (name, value) = s.split_at(index);
        Some((name.to_string(), value[1..].to_string()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::certificate::Certificate;
    use crate::http::easy_ext::CertInfo;

    #[test]
    fn test_parse_start_date() {
        let mut attributes = HashMap::new();
        attributes.insert(
            "start date".to_string(),
            "Jan 10 08:29:52 2023 GMT".to_string(),
        );
        assert_eq!(
            parse_start_date(&attributes).unwrap(),
            chrono::DateTime::parse_from_rfc2822("Tue, 10 Jan 2023 08:29:52 GMT")
                .unwrap()
                .with_timezone(&chrono::Utc)
        );

        let mut attributes = HashMap::new();
        attributes.insert(
            "start date".to_string(),
            "2023-01-10 08:29:52 GMT".to_string(),
        );
        assert_eq!(
            parse_start_date(&attributes).unwrap(),
            chrono::DateTime::parse_from_rfc2822("Tue, 10 Jan 2023 08:29:52 GMT")
                .unwrap()
                .with_timezone(&chrono::Utc)
        )
    }

    #[test]
    fn test_try_from() {
        assert_eq!(
            Certificate::try_from(CertInfo {
                data: vec![
                    "Subject:C = US, ST = Denial, L = Springfield, O = Dis, CN = localhost"
                        .to_string(),
                    "Issuer:C = US, ST = Denial, L = Springfield, O = Dis, CN = localhost"
                        .to_string(),
                    "Serial Number:1ee8b17f1b64d8d6b3de870103d2a4f533535ab0".to_string(),
                    "Start date:Jan 10 08:29:52 2023 GMT".to_string(),
                    "Expire date:Oct 30 08:29:52 2025 GMT".to_string(),
                ]
            })
            .unwrap(),
            Certificate {
                subject: "C = US, ST = Denial, L = Springfield, O = Dis, CN = localhost"
                    .to_string(),
                issuer: "C = US, ST = Denial, L = Springfield, O = Dis, CN = localhost".to_string(),
                start_date: chrono::DateTime::parse_from_rfc2822("Tue, 10 Jan 2023 08:29:52 GMT")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                expire_date: chrono::DateTime::parse_from_rfc2822("Thu, 30 Oct 2025 08:29:52 GMT")
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                serial_number: "1ee8b17f1b64d8d6b3de870103d2a4f533535ab0".to_string()
            }
        );
        assert_eq!(
            Certificate::try_from(CertInfo { data: vec![] })
                .err()
                .unwrap(),
            "missing Subject attribute in {}".to_string()
        );
    }
}
