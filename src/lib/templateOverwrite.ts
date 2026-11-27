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

/**
 * What to tell the user when a duplicate upload is refused (HTTP 409) on a server type
 * that can't overwrite. FerroEHR is specific: per its documentation templates are
 * immutable once loaded, and only an admin extension can delete one (refused while any
 * composition still uses it). For the other types nothing has been verified, so the
 * message stays general.
 */
export function duplicateWithoutOverwriteMessage(serverType: ServerType | undefined): string {
  const exists = "A template with this ID already exists on the server.";
  if (serverType === "ferro_ehr") {
    return `${exists} FerroEHR templates are immutable, so it can't be overwritten: upload it under a new template ID or version, or have an admin delete the old one first (only possible if no composition uses it).`;
  }
  if (serverType === "better_platform") {
    return "An identical template with this ID is already installed on the server, so nothing changed. Better versions templates: uploading a changed template under the same ID adds a new active version.";
  }
  return `${exists} Overwriting isn't supported for this server type; remove the existing template on the server first.`;
}

/**
 * Whether uploading a template whose ID already exists might silently replace it, so the
 * user should be asked first. Servers differ: the EHRbase sandbox replaces on a plain
 * upload, others answer 409. The question is pointless on FerroEHR, which documents
 * templates as immutable once loaded and always refuses, and on Better Platform, which
 * versions templates: a changed upload becomes a new active version (nothing is lost)
 * and an identical one answers 409 without changing anything.
 */
export function duplicateMayReplace(serverType: ServerType | undefined): boolean {
  return serverType !== "ferro_ehr" && serverType !== "better_platform";
}

/**
 * Whether a template can be deleted on this server type. The openEHR REST API has no
 * template delete, so it is a vendor admin route (ADR-0030): EHRbase
 * `DELETE /rest/admin/template/{id}` and FerroEHR's admin extension. Better Platform has
 * not been checked, so nothing is offered there.
 */
export function supportsTemplateDelete(serverType: ServerType | undefined): boolean {
  return serverType === "ehrbase" || serverType === "ferro_ehr";
}
