import type { ServerProfile } from "../stores/server";

type ServerType = ServerProfile["server_type"];

/**
 * Whether an existing template can be replaced on this server type.
 *
 * The openEHR REST API only defines POST for templates (a duplicate answers 409), so
 * overwriting is vendor-specific (ADR-0030). Only EHRbase is implemented: its Admin
 * API offers `PUT /rest/admin/template/{id}`. For the other types nothing has been
 * verified, so no overwrite is offered rather than sending a guessed request.
 */
export function supportsTemplateOverwrite(serverType: ServerType | undefined): boolean {
  return serverType === "ehrbase";
}
