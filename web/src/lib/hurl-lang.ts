// Hurl syntax rules adapted from https://github.com/alextreichler/hurl-syntax-highlighting
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

        // Methods
        [/\b(GET|HEAD|POST|PUT|DELETE|CONNECT|OPTIONS|TRACE|PATCH)\b/, 'keyword.control'],

        // Operators
        [/[\-\^=\*\|><]/, 'keyword.operator'],

        // Query functions and sections
        [/\b(status|url|header|cookie|body|xpath|jsonpath|regex|variable|duration|sha256|md5|bytes)\b/, 'support.function'],
        [/\b(Asserts|FormParams|Options|Captures|QueryStringParams|MultipartFormData|Cookies)\b/, 'constant.character'],

        // Strings (double-quoted only, to match reference grammar)
        [/"([^"\\]|\\.)*"/, 'string'],

        // Numbers
        [/\b(?:\d[_\d]*\.\d[_\d]*(?:[eE][-+]?\d[_\d]*)?|\d[_\d]*[eE][-+]?\d[_\d]*)\b/, 'number.float'],
        [/\b(?:0[xX]_*[0-9a-fA-F][0-9a-fA-F_]*\.[0-9a-fA-F][0-9a-fA-F_]*(?:[pP][-+]?\d[_\d]*)?|0[xX]_*[0-9a-fA-F][0-9a-fA-F_]*[pP][-+]?\d[_\d]*)\b/, 'number.hex'],
        [/\b0[xX]_*[0-9a-fA-F][0-9a-fA-F_]*\b/, 'number.hex'],
        [/\b0[oO]_*[0-7][0-7_]*\b/, 'number.octal'],
        [/\b0[bB]_*[01][01_]*\b/, 'number.binary'],
        [/\b\d[_\d]*\b/, 'number'],

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
