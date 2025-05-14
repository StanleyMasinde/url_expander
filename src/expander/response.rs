use http_body_util::BodyExt;
use http_body_util::{Full, combinators::BoxBody};
use hyper::{
    Error, Response, StatusCode,
    body::Bytes,
    header::{self, HeaderValue},
};

pub fn build_response(
    status: StatusCode,
    body: String,
) -> Result<Response<BoxBody<Bytes, Error>>, Error> {
    let mut bad_request = Response::new(full(body));
    *bad_request.status_mut() = status;
    hyper::HeaderMap::insert(
        bad_request.headers_mut(),
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );

    Ok(bad_request)
}

/// Return full response
fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{header, StatusCode};
    use http_body_util::BodyExt;

    // Helper function to extract the body as a String
    async fn extract_body(mut response: Response<BoxBody<Bytes, Error>>) -> String {
        let body_bytes = response.body_mut().collect().await.unwrap().to_bytes();
        String::from_utf8(body_bytes.to_vec()).unwrap()
    }

    #[tokio::test]
    async fn test_build_response_200_ok() {
        let response = build_response(StatusCode::OK, "Hello, world!".to_string()).unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body_content = extract_body(response).await;
        assert_eq!(body_content, "Hello, world!");

        let response = build_response(StatusCode::OK, "Hello, world!".to_string()).unwrap();
        assert_eq!(
            response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
            "*"
        );
    }

    #[tokio::test]
    async fn test_build_response_404_not_found() {
        let response = build_response(StatusCode::NOT_FOUND, "Not found".to_string()).unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body_content = extract_body(response).await;
        assert_eq!(body_content, "Not found");

        let response = build_response(StatusCode::NOT_FOUND, "Not found".to_string()).unwrap();
        assert_eq!(
            response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(),
            "*"
        );
    }
}
