/**
 * AQL Query Formatter
 *
 * Formats AQL queries with SQL-style best practices:
 * - Keywords in UPPERCASE
 * - Proper indentation for clauses
 * - One clause per line for major keywords
 * - Aligned column lists in SELECT
 */

const MAJOR_KEYWORDS = [
  'SELECT',
  'FROM',
  'WHERE',
  'CONTAINS',
  'ORDER BY',
  'GROUP BY',
  'HAVING',
  'LIMIT',
  'OFFSET',
  'UNION',
];

const ALL_KEYWORDS = [
  ...MAJOR_KEYWORDS,
  'AND',
  'OR',
  'NOT',
  'AS',
  'LIKE',
  'MATCHES',
  'EXISTS',
  'TOP',
  'DISTINCT',
  'ASC',
  'DESC',
  'COUNT',
  'MAX',
  'MIN',
  'SUM',
  'AVG',
];

const RM_TYPES = [
  'EHR',
  'COMPOSITION',
  'OBSERVATION',
  'EVALUATION',
  'INSTRUCTION',
  'ACTION',
  'ADMIN_ENTRY',
  'CLUSTER',
  'ELEMENT',
  'SECTION',
  'PARTY_PROXY',
  'PARTY_IDENTIFIED',
  'PARTY_RELATED',
  'PARTY_SELF',
];

/**
 * Format an AQL query with SQL-style formatting
 */
export function formatAql(query: string): string {
  if (!query.trim()) return query;

  // Step 1: Normalize whitespace
  let formatted = query.replace(/\s+/g, ' ').trim();

  // Step 2: Capitalize keywords
  const keywordPattern = new RegExp(`\\b(${ALL_KEYWORDS.join('|')})\\b`, 'gi');
  formatted = formatted.replace(keywordPattern, (match) => match.toUpperCase());

  // Step 3: Capitalize RM types
  const rmTypePattern = new RegExp(`\\b(${RM_TYPES.join('|')})\\b`, 'gi');
  formatted = formatted.replace(rmTypePattern, (match) => match.toUpperCase());

  // Step 4: Add line breaks before major keywords
  // Special handling for ORDER BY and GROUP BY (two-word keywords)
  formatted = formatted.replace(/\bORDER\s+BY\b/gi, '\nORDER BY');
  formatted = formatted.replace(/\bGROUP\s+BY\b/gi, '\nGROUP BY');

  // Single-word major keywords
  const singleWordMajor = MAJOR_KEYWORDS.filter(k => !k.includes(' '));
  const majorPattern = new RegExp(`\\b(${singleWordMajor.join('|')})\\b`, 'gi');
  formatted = formatted.replace(majorPattern, (match) => '\n' + match.toUpperCase());

  // Step 5: Format SELECT clause - put each column on its own line with indentation
  formatted = formatted.replace(
    /SELECT\s+(.+?)(?=\nFROM|\nWHERE|\nCONTAINS|$)/is,
    (match, columns) => {
      // Split by comma, but not commas inside parentheses or strings
      const columnList = splitByComma(columns.trim());
      if (columnList.length > 1) {
        const indentedColumns = columnList
          .map((col, i) => (i === 0 ? col.trim() : '  ' + col.trim()))
          .join(',\n');
        return 'SELECT ' + indentedColumns;
      }
      return match;
    }
  );

  // Step 6: Indent CONTAINS clauses
  formatted = formatted.replace(/\nCONTAINS\s+/g, '\n  CONTAINS ');

  // Step 7: Format WHERE conditions with proper AND/OR indentation
  formatted = formatted.replace(
    /WHERE\s+(.+?)(?=\nORDER BY|\nGROUP BY|\nLIMIT|\nOFFSET|$)/is,
    (match, conditions) => {
      // Add line breaks before AND/OR with indentation
      const formattedConditions = conditions
        .replace(/\s+AND\s+/gi, '\n    AND ')
        .replace(/\s+OR\s+/gi, '\n    OR ');
      return 'WHERE ' + formattedConditions.trim();
    }
  );

  // Step 8: Clean up multiple line breaks
  formatted = formatted.replace(/\n{3,}/g, '\n\n').trim();

  // Step 9: Ensure first line doesn't start with newline
  formatted = formatted.replace(/^\n+/, '');

  return formatted;
}

/**
 * Split a string by commas, but ignore commas inside parentheses or quotes
 */
function splitByComma(str: string): string[] {
  const result: string[] = [];
  let current = '';
  let depth = 0;
  let inQuote = false;
  let quoteChar = '';

  for (let i = 0; i < str.length; i++) {
    const char = str[i];

    if ((char === '"' || char === "'") && (i === 0 || str[i - 1] !== '\\')) {
      if (!inQuote) {
        inQuote = true;
        quoteChar = char;
      } else if (char === quoteChar) {
        inQuote = false;
      }
    }

    if (!inQuote) {
      if (char === '(') depth++;
      else if (char === ')') depth--;
    }

    if (char === ',' && depth === 0 && !inQuote) {
      result.push(current);
      current = '';
    } else {
      current += char;
    }
  }

  if (current.trim()) {
    result.push(current);
  }

  return result;
}
