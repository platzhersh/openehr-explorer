import { sql, SQLDialect } from "@codemirror/lang-sql";

/**
 * AQL-specific keywords (in addition to SQL keywords)
 */
export const AQL_KEYWORDS = [
  "CONTAINS",
  "MATCHES",
  "EXISTS",
  "EHR",
  "COMPOSITION",
  "OBSERVATION",
  "EVALUATION",
  "INSTRUCTION",
  "ACTION",
  "ADMIN_ENTRY",
  "CLUSTER",
  "ELEMENT",
  "SECTION",
];

/**
 * The standard SQL keyword/type lists @codemirror/lang-sql's `StandardSQL`
 * dialect uses by default. `StandardSQL` is defined with an empty spec
 * (`SQLDialect.define({})`), so `StandardSQL.spec.keywords`/`.types` are
 * `undefined` — the real lists only exist internally as the dialect's
 * default keyword table. `SQLDialect.define` replaces that table entirely
 * whenever a `keywords` string is supplied, so we have to spell out the
 * standard keywords/types ourselves here rather than layering AQL keywords
 * on top of `StandardSQL.spec` (which would silently drop SELECT/FROM/WHERE
 * and friends). Copied from @codemirror/lang-sql@6.10.0's internal
 * `SQLKeywords`/`SQLTypes` constants — this project pins exact dependency
 * versions, so keep this in sync if that version ever changes.
 */
const STANDARD_SQL_KEYWORDS =
  "absolute action add after all allocate alter and any are as asc assertion at authorization before begin between both breadth by call cascade cascaded case cast catalog check close collate collation column commit condition connect connection constraint constraints constructor continue corresponding count create cross cube current current_date current_default_transform_group current_transform_group_for_type current_path current_role current_time current_timestamp current_user cursor cycle data day deallocate declare default deferrable deferred delete depth deref desc describe descriptor deterministic diagnostics disconnect distinct do domain drop dynamic each else elseif end end-exec equals escape except exception exec execute exists exit external fetch first for foreign found from free full function general get global go goto grant group grouping handle having hold hour identity if immediate in indicator initially inner inout input insert intersect into is isolation join key language last lateral leading leave left level like limit local localtime localtimestamp locator loop map match method minute modifies module month names natural nesting new next no none not of old on only open option or order ordinality out outer output overlaps pad parameter partial path prepare preserve primary prior privileges procedure public read reads recursive redo ref references referencing relative release repeat resignal restrict result return returns revoke right role rollback rollup routine row rows savepoint schema scroll search second section select session session_user set sets signal similar size some space specific specifictype sql sqlexception sqlstate sqlwarning start state static system_user table temporary then timezone_hour timezone_minute to trailing transaction translation treat trigger under undo union unique unnest until update usage user using value values view when whenever where while with without work write year zone";
const STANDARD_SQL_TYPES =
  "array binary bit boolean char character clob date decimal double float int integer interval large national nchar nclob numeric object precision real smallint time timestamp varchar varying";

/**
 * Standard SQL dialect extended with openEHR/AQL-specific keywords, so the
 * tokenizer highlights them the same way it highlights standard SQL
 * keywords (SELECT, FROM, WHERE, ...) rather than rendering them as plain
 * identifiers. Keywords are tagged the same as standard SQL keywords, so
 * the existing highlight style picks them up without further changes.
 */
const aqlDialect = SQLDialect.define({
  keywords: `${STANDARD_SQL_KEYWORDS} ${AQL_KEYWORDS.join(" ").toLowerCase()}`,
  types: STANDARD_SQL_TYPES,
});

/**
 * AQL language support for CodeMirror.
 * AQL is SQL-like with openEHR-specific keywords and path syntax.
 */
export function aql() {
  // Use the standard SQL dialect as base (AQL is SQL-like), extended with
  // AQL-specific keywords so they're tokenized/highlighted as keywords too.
  return sql({ dialect: aqlDialect });
}
