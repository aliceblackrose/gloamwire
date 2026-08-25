use reqwest::{Method, header::HeaderMap};

use crate::{
    Error, Result,
    model::{
        ApplicationId, InteractionCallbackData, InteractionCallbackResponse, InteractionId,
        InteractionMessageData, InteractionResponse, Message, MessageId,
    },
};

use super::{
    EditWebhookMessage, RestClient, UploadFile,
    encoding::{QueryBuilder, percent_encode},
    route::{RetrySafety, Route},
};

/// Query options for creating an interaction callback.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CreateInteractionResponseQuery {
    pub with_response: Option<bool>,
}

impl CreateInteractionResponseQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(with_response) = self.with_response {
            query.push("with_response", with_response);
        }
        query.finish()
    }
}

/// Query options for editing an original response or followup message.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EditInteractionMessageQuery {
    pub with_components: Option<bool>,
}

impl EditInteractionMessageQuery {
    fn suffix(&self) -> String {
        let mut query = QueryBuilder::default();
        if let Some(with_components) = self.with_components {
            query.push("with_components", with_components);
        }
        query.finish()
    }
}

impl RestClient {
    /// Sends the initial response to an interaction.
    ///
    /// The returned value is `Some` only when `with_response` asks Discord to
    /// return the created callback resource.
    pub async fn create_interaction_response(
        &self,
        interaction_id: InteractionId,
        token: &str,
        response: &InteractionResponse,
        query: &CreateInteractionResponseQuery,
    ) -> Result<Option<InteractionCallbackResponse>> {
        self.request_optional_json(
            interaction_callback_route(interaction_id, token, &query.suffix(), RetrySafety::Unsafe),
            Some(response),
            HeaderMap::new(),
        )
        .await
    }

    /// Sends the initial interaction response with streamed file uploads.
    pub async fn create_interaction_response_with_files(
        &self,
        interaction_id: InteractionId,
        token: &str,
        response: &InteractionResponse,
        files: &[UploadFile],
        query: &CreateInteractionResponseQuery,
    ) -> Result<Option<InteractionCallbackResponse>> {
        let mut request = response.clone();
        let Some(InteractionCallbackData::Message(message)) = &mut request.data else {
            return Err(Error::InvalidRestRequest(
                "interaction callback files require message callback data".to_owned(),
            ));
        };
        message
            .attachments
            .extend(files.iter().map(UploadFile::attachment_request));

        self.request_optional_multipart_json(
            interaction_callback_route(interaction_id, token, &query.suffix(), RetrySafety::Unsafe),
            &request,
            files,
            HeaderMap::new(),
        )
        .await
    }

    /// Returns the initial response to an interaction.
    pub async fn get_original_interaction_response(
        &self,
        application_id: ApplicationId,
        token: &str,
    ) -> Result<Message> {
        self.request_json::<Message, ()>(
            interaction_message_route(
                Method::GET,
                application_id,
                token,
                "@original",
                "",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Edits the initial response to an interaction.
    pub async fn edit_original_interaction_response(
        &self,
        application_id: ApplicationId,
        token: &str,
        edit: &EditWebhookMessage,
        query: &EditInteractionMessageQuery,
    ) -> Result<Message> {
        self.request_json(
            interaction_message_route(
                Method::PATCH,
                application_id,
                token,
                "@original",
                &query.suffix(),
                RetrySafety::Unsafe,
            ),
            Some(edit),
        )
        .await
    }

    /// Edits the initial interaction response and appends file uploads.
    pub async fn edit_original_interaction_response_with_files(
        &self,
        application_id: ApplicationId,
        token: &str,
        edit: &EditWebhookMessage,
        files: &[UploadFile],
        query: &EditInteractionMessageQuery,
    ) -> Result<Message> {
        self.edit_interaction_message_with_files(
            application_id,
            token,
            "@original",
            edit,
            files,
            query,
        )
        .await
    }

    /// Deletes the initial response to an interaction.
    pub async fn delete_original_interaction_response(
        &self,
        application_id: ApplicationId,
        token: &str,
    ) -> Result<()> {
        self.request_empty::<()>(
            interaction_message_route(
                Method::DELETE,
                application_id,
                token,
                "@original",
                "",
                RetrySafety::Safe,
            ),
            None,
            HeaderMap::new(),
        )
        .await
    }

    /// Creates an interaction followup message.
    pub async fn create_followup_message(
        &self,
        application_id: ApplicationId,
        token: &str,
        message: &InteractionMessageData,
    ) -> Result<Message> {
        self.request_json(
            interaction_webhook_route(
                Method::POST,
                application_id,
                token,
                "",
                "/webhooks/{application_id}/{interaction_token}",
                RetrySafety::Unsafe,
            ),
            Some(message),
        )
        .await
    }

    /// Creates an interaction followup with streamed file uploads.
    pub async fn create_followup_message_with_files(
        &self,
        application_id: ApplicationId,
        token: &str,
        message: &InteractionMessageData,
        files: &[UploadFile],
    ) -> Result<Message> {
        let mut request = message.clone();
        request
            .attachments
            .extend(files.iter().map(UploadFile::attachment_request));

        self.request_multipart_json(
            interaction_webhook_route(
                Method::POST,
                application_id,
                token,
                "",
                "/webhooks/{application_id}/{interaction_token}",
                RetrySafety::Unsafe,
            ),
            &request,
            files,
            HeaderMap::new(),
        )
        .await
    }

    /// Returns one interaction followup message.
    pub async fn get_followup_message(
        &self,
        application_id: ApplicationId,
        token: &str,
        message_id: MessageId,
    ) -> Result<Message> {
        self.request_json::<Message, ()>(
            interaction_message_route(
                Method::GET,
                application_id,
                token,
                &message_id.to_string(),
                "",
                RetrySafety::Safe,
            ),
            None,
        )
        .await
    }

    /// Edits one interaction followup message.
    pub async fn edit_followup_message(
        &self,
        application_id: ApplicationId,
        token: &str,
        message_id: MessageId,
        edit: &EditWebhookMessage,
        query: &EditInteractionMessageQuery,
    ) -> Result<Message> {
        self.request_json(
            interaction_message_route(
                Method::PATCH,
                application_id,
                token,
                &message_id.to_string(),
                &query.suffix(),
                RetrySafety::Unsafe,
            ),
            Some(edit),
        )
        .await
    }

    /// Edits one interaction followup and appends file uploads.
    pub async fn edit_followup_message_with_files(
        &self,
        application_id: ApplicationId,
        token: &str,
        message_id: MessageId,
        edit: &EditWebhookMessage,
        files: &[UploadFile],
        query: &EditInteractionMessageQuery,
    ) -> Result<Message> {
        self.edit_interaction_message_with_files(
            application_id,
            token,
            &message_id.to_string(),
            edit,
            files,
            query,
        )
        .await
    }

    /// Deletes one interaction followup message.
    pub async fn delete_followup_message(
        &self,
        application_id: ApplicationId,
        token: &str,
        message_id: MessageId,
    ) -> Result<()> {
        self.request_empty::<()>(
            interaction_message_route(
                Method::DELETE,
                application_id,
                token,
                &message_id.to_string(),
                "",
                RetrySafety::Safe,
            ),
            None,
            HeaderMap::new(),
        )
        .await
    }

    async fn edit_interaction_message_with_files(
        &self,
        application_id: ApplicationId,
        token: &str,
        message: &str,
        edit: &EditWebhookMessage,
        files: &[UploadFile],
        query: &EditInteractionMessageQuery,
    ) -> Result<Message> {
        let mut request = edit.clone();
        request
            .attachments
            .get_or_insert_with(|| Some(Vec::new()))
            .get_or_insert_default()
            .extend(files.iter().map(UploadFile::attachment_request));

        self.request_multipart_json(
            interaction_message_route(
                Method::PATCH,
                application_id,
                token,
                message,
                &query.suffix(),
                RetrySafety::Unsafe,
            ),
            &request,
            files,
            HeaderMap::new(),
        )
        .await
    }
}

fn interaction_callback_route(
    interaction_id: InteractionId,
    token: &str,
    suffix: &str,
    safety: RetrySafety,
) -> Route {
    Route::new(
        Method::POST,
        format!(
            "/interactions/{interaction_id}/{}/callback{suffix}",
            percent_encode(token)
        ),
        "/interactions/{interaction_id}/{interaction_token}/callback",
        Some(interaction_id.to_string()),
        safety,
    )
}

fn interaction_webhook_route(
    method: Method,
    application_id: ApplicationId,
    token: &str,
    suffix: &str,
    template: &'static str,
    safety: RetrySafety,
) -> Route {
    Route::new(
        method,
        format!(
            "/webhooks/{application_id}/{}{suffix}",
            percent_encode(token)
        ),
        template,
        Some(application_id.to_string()),
        safety,
    )
}

fn interaction_message_route(
    method: Method,
    application_id: ApplicationId,
    token: &str,
    message: &str,
    suffix: &str,
    safety: RetrySafety,
) -> Route {
    interaction_webhook_route(
        method,
        application_id,
        token,
        &format!("/messages/{message}{suffix}"),
        "/webhooks/{application_id}/{interaction_token}/messages/{message_id}",
        safety,
    )
}

#[cfg(test)]
mod tests {
    use crate::{
        Error,
        model::{
            ApplicationId, InteractionCallbackType, InteractionId, InteractionResponse, Modal,
        },
    };

    use super::{CreateInteractionResponseQuery, RestClient, interaction_message_route};
    use crate::http::route::RetrySafety;
    use reqwest::Method;

    #[test]
    fn callback_query_serializes_with_response() {
        let query = CreateInteractionResponseQuery {
            with_response: Some(true),
        };

        assert_eq!(query.suffix(), "?with_response=true");
    }

    #[tokio::test]
    async fn callback_uploads_require_message_data() {
        let client = RestClient::new("token").expect("client");
        let response = InteractionResponse {
            kind: InteractionCallbackType::MODAL,
            data: Some(crate::model::InteractionCallbackData::Modal(Modal {
                custom_id: "settings".to_owned(),
                title: "Settings".to_owned(),
                components: Vec::new(),
            })),
        };

        let result = client
            .create_interaction_response_with_files(
                InteractionId::new(1),
                "token",
                &response,
                &[crate::http::UploadFile::bytes(0, "file.txt", b"data")],
                &CreateInteractionResponseQuery::default(),
            )
            .await;

        assert!(matches!(result, Err(Error::InvalidRestRequest(_))));
    }

    #[test]
    fn interaction_tokens_are_encoded_as_path_segments() {
        let route = interaction_message_route(
            Method::GET,
            ApplicationId::new(1),
            "token/with space",
            "@original",
            "",
            RetrySafety::Safe,
        );

        assert_eq!(
            route.path,
            "/webhooks/1/token%2Fwith%20space/messages/@original"
        );
    }
}
