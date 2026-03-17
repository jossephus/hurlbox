import type * as monaco from 'monaco-editor'

export function registerHurlLanguage(monacoInstance: typeof monaco) {
  monacoInstance.languages.register({ id: 'hurl' })

  monacoInstance.languages.setMonarchTokensProvider('hurl', {
    ignoreCase: true,
    defaultToken: '',
    
    tokenizer: {
      root: [
        // Comments
        [/#.*$/, 'comment'],
        
        // Section headers
        [/\[Asserts\]/, 'keyword.control'],
        [/\[Captures\]/, 'keyword.control'],
        [/\[QueryStringParams\]/, 'keyword.control'],
        [/\[FormParams\]/, 'keyword.control'],
        [/\[MultipartFormData\]/, 'keyword.control'],
        [/\[Cookies\]/, 'keyword.control'],
        [/\[Options\]/, 'keyword.control'],
        
        // HTTP version and status
        [/HTTP\/[\d.]+/, 'keyword.control'],
        [/\bHTTP\s+\d+/, 'keyword.control'],
        
        // Methods
        [/\b(GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS)\b/, 'keyword.other.http-method'],
        
        // URL
        [/(https?:\/\/[^\s]+)/, 'string.url'],
        
        // Strings
        [/"([^"\\]|\\.)*"/, 'string'],
        [/'([^'\\]|\\.)*'/, 'string'],
        
        // Variables {{var}}
        [/\{\{[^}]+\}\}/, 'variable'],
        
        // JSONPath
        [/\$[.\[\]"']*/, 'variable.other.jsonpath'],
        
        // XPath
        [/\bxpath\b/, 'keyword.control'],
        
        // Queries
        [/\b(status|body|header|jsonpath|xpath|regex|url|duration|bytes|sha256|md5|variable)\b/, 'keyword.control'],
        
        // Predicates
        [/\b(contains|exists|isInteger|isFloat|isBoolean|isCollection|isEmpty|notEquals|equals|startsWith|endsWith|matches|include|isString|lessThan|greaterThan|lessThanOrEqual|greaterThanOrEqual|between)\b/, 'keyword.operator'],
        
        // Boolean
        [/\b(true|false)\b/, 'constant.language.boolean'],
        
        // Numbers
        [/\b\d+\b/, 'number'],
        
        // Filenames
        [/\bfile\b/, 'keyword.control'],
        
        // Base64
        [/\bbase64\b/, 'keyword.control'],
        
        // Helpers
        [/\b(date|now|uuid)\b/, 'support.function'],
        
        // Whitespace
        [/\s+/, 'white'],
      ],
    },
  })

  monacoInstance.languages.setLanguageConfiguration('hurl', {
    comments: {
      lineComment: '#',
    },
    brackets: [
      ['{', '}'],
      ['[', ']'],
      ['(', ')'],
    ],
    autoClosingPairs: [
      { open: '{', close: '}' },
      { open: '[', close: ']' },
      { open: '(', close: ')' },
      { open: '"', close: '"' },
      { open: "'", close: "'" },
    ],
  })
}