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

use infra::{
    db::{ORM_CLIENT, connect_to_orm},
    errors::Result,
    table,
};
use o2_enterprise::enterprise::super_cluster::queue::{AlertMessage, Message, MessageType};

pub(crate) async fn process(msg: Message) -> Result<()> {
    match msg.message_type {
        MessageType::AlertsTable => {
            let msg = msg.try_into()?;
            process_msg(msg).await?;
        }
        _ => {
            // Try to process the message as an old event for the meta table for backward
            // compatability. This logic can be removed after all logic reading and writing alerts
            // to the meta table is removed from the application.
            super::meta::process(msg).await?;
        }
    }

    Ok(())
}

pub(crate) async fn process_msg(msg: AlertMessage) -> Result<()> {
    let conn = ORM_CLIENT.get_or_init(connect_to_orm).await;
    match msg {
        AlertMessage::Create {
            org_id,
            folder_id,
            alert,
        } => {
            if table::alerts::get_by_id(conn, &org_id, alert.id.expect("alert id cannot be none"))
                .await?
                .is_some()
            {
                return Ok(());
            }
            table::alerts::create(conn, &org_id, &folder_id, alert.clone(), true).await?;
            infra::cluster_coordinator::alerts::emit_put_event(&org_id, &alert, Some(folder_id))
                .await?;
        }
        AlertMessage::Update {
            org_id,
            folder_id,
            alert,
        } => {
            let alert = table::alerts::update(conn, &org_id, folder_id.as_deref(), alert).await?;
            infra::cluster_coordinator::alerts::emit_put_event(&org_id, &alert, folder_id).await?;
        }
        AlertMessage::Delete { org_id, alert_id } => {
            if table::alerts::get_by_id(conn, &org_id, alert_id)
                .await?
                .is_some()
            {
                table::alerts::delete_by_id(conn, &org_id, alert_id).await?;
                infra::cluster_coordinator::alerts::emit_delete_event(
                    &org_id,
                    &alert_id.to_string(),
                )
                .await?;
            }
        }
    };
    Ok(())
}
