export interface RmPathEntry {
  path: string;
  rmType: string;
  description: string;
}

export const EHR_PATHS: RmPathEntry[] = [
  {
    path: "ehr_id/value",
    rmType: "String",
    description: "UUID of the EHR",
  },
  {
    path: "time_created/value",
    rmType: "DV_DATE_TIME",
    description: "Creation timestamp",
  },
  {
    path: "system_id/value",
    rmType: "String",
    description: "Owning CDR system identifier",
  },
  {
    path: "ehr_status/subject/external_ref/id/value",
    rmType: "String",
    description: "Patient identifier",
  },
  {
    path: "ehr_status/subject/external_ref/namespace",
    rmType: "String",
    description: "Patient ID namespace",
  },
  {
    path: "ehr_status/is_queryable",
    rmType: "Boolean",
    description: "Whether EHR appears in AQL",
  },
  {
    path: "ehr_status/is_modifiable",
    rmType: "Boolean",
    description: "Whether EHR accepts writes",
  },
  {
    path: "ehr_status/uid/value",
    rmType: "String",
    description: "UID of the EHR_STATUS object",
  },
];

export const COMPOSITION_PATHS: RmPathEntry[] = [
  {
    path: "uid/value",
    rmType: "String",
    description: "Composition UID",
  },
  {
    path: "name/value",
    rmType: "String",
    description: "Composition name (template name)",
  },
  {
    path: "archetype_details/template_id/value",
    rmType: "String",
    description: "Template ID",
  },
  {
    path: "archetype_details/archetype_id/value",
    rmType: "String",
    description: "Root archetype ID",
  },
  {
    path: "archetype_details/rm_version",
    rmType: "String",
    description: "RM version string",
  },
  {
    path: "context/start_time/value",
    rmType: "DV_DATE_TIME",
    description: "Composition start time",
  },
  {
    path: "context/end_time/value",
    rmType: "DV_DATE_TIME",
    description: "Composition end time",
  },
  {
    path: "context/location",
    rmType: "String",
    description: "Clinical location",
  },
  {
    path: "context/setting/value",
    rmType: "String",
    description: 'Setting (e.g., "primary medical care")',
  },
  {
    path: "language/code_string",
    rmType: "String",
    description: "ISO 639-1 language code",
  },
  {
    path: "territory/code_string",
    rmType: "String",
    description: "ISO 3166-1 territory code",
  },
  {
    path: "composer/name",
    rmType: "String",
    description: "Author name",
  },
];

/** Map RM type names to their static path tables */
export const RM_PATH_MAP: Record<string, RmPathEntry[]> = {
  EHR: EHR_PATHS,
  COMPOSITION: COMPOSITION_PATHS,
};
