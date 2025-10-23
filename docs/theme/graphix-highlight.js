/**
 * Syntax highlighting definition for the Graphix programming language
 * For use with highlight.js in mdbook
 */

console.log('Graphix highlighter: Script loaded');

(function() {
  'use strict';

  console.log('Graphix highlighter: IIFE started');

  function graphix(hljs) {
    // Type parameter pattern: 'a, 'b, 'r, 'e, etc.
    const TYPE_PARAM = {
      className: 'type',
      begin: "'[a-z][a-z0-9_]*\\b"
    };

    // Variant pattern: `Foo, `Bar, `MoreArg, etc.
    const VARIANT = {
      className: 'symbol',
      begin: '`[A-Z][a-zA-Z0-9_]*'
    };

    // Labeled argument pattern: #label:
    const LABEL = {
      className: 'attr',
      begin: '#[a-z_][a-zA-Z0-9_]*:',
      relevance: 0
    };

    // Reference operator: &
    const REFERENCE = {
      className: 'operator',
      begin: '&[a-z_]',
      relevance: 0
    };

    // Operators
    const OPERATORS = {
      className: 'operator',
      begin: '(<-|->|=>|~|\\?|\\$|\\.\\.|::|@|\\*(?=[a-z_])|\\||\\+|\\-|/|%|==|!=|<=|>=|<|>|&&|\\|\\|)'
    };

    // Numbers with optional type suffixes
    const NUMBER = {
      className: 'number',
      variants: [
        // Hexadecimal
        { begin: '\\b0x[0-9a-fA-F]+(_?[0-9a-fA-F]+)*(i32|z32|i64|z64|u32|v32|u64|v64)?\\b' },
        // Decimal/Float with type suffix
        { begin: '\\b\\d+(_?\\d+)*\\.\\d+(_?\\d+)*([eE][+-]?\\d+(_?\\d+)*)?(f32|f64)?\\b' },
        // Float with exponent
        { begin: '\\b\\d+(_?\\d+)*[eE][+-]?\\d+(_?\\d+)*(f32|f64)?\\b' },
        // Integer with type suffix
        { begin: '\\b\\d+(_?\\d+)*(i32|z32|i64|z64|u32|v32|u64|v64|f32|f64)?\\b' }
      ],
      relevance: 0
    };

    // Duration literals: 1.5s, 500ms, etc.
    const DURATION = {
      className: 'number',
      begin: '\\b\\d+(\\.\\d+)?(ms|s|m|h|d)\\b'
    };

    // String with interpolation support
    const STRING = {
      className: 'string',
      variants: [
        {
          begin: '"',
          end: '"',
          contains: [
            {
              className: 'subst',
              begin: '\\[',
              end: '\\]',
              contains: ['self']
            },
            {
              className: 'char.escape',
              begin: '\\\\.',
              relevance: 0
            }
          ]
        }
      ]
    };

    // Module path pattern: array::map, net::subscribe, etc.
    const MODULE_PATH = {
      className: 'title.function',
      begin: '\\b[a-z_][a-z0-9_]*::[a-z_][a-z0-9_]*\\b'
    };

    // Function call pattern
    const FUNCTION_CALL = {
      className: 'title.function',
      begin: '\\b[a-z_][a-z0-9_]*(?=\\()',
      relevance: 0
    };

    // Type names: Array, Map, String, Result, Option, Error, etc.
    const TYPE_NAME = {
      className: 'type',
      begin: '\\b[A-Z][a-zA-Z0-9_]*\\b',
      relevance: 0
    };

    return {
      name: 'Graphix',
      aliases: ['gx'],
      keywords: {
        keyword:
          'let fn mod type val sig use select if throws dynamic sandbox whitelist as with',
        literal:
          'true false null',
        built_in:
          // Core functions
          'print println log dbg error never all and count divide filter_err filter ' +
          'is_err max mean min once seq or product sum uniq queue hold throttle cast ' +
          // Common type names that should be highlighted
          'Array Map Result Option Error String Number Int Float Bool DateTime Duration'
      },
      contains: [
        // Documentation comments (must come before regular comments)
        {
          className: 'comment',
          begin: '///',
          end: '$',
          relevance: 10
        },
        // Regular comments
        hljs.COMMENT('//', '$'),
        // Block comments
        hljs.COMMENT('/\\*', '\\*/'),

        STRING,
        VARIANT,
        TYPE_PARAM,
        LABEL,
        MODULE_PATH,
        FUNCTION_CALL,
        TYPE_NAME,
        DURATION,
        NUMBER,
        OPERATORS,
        REFERENCE,

        // Lambda syntax
        {
          className: 'function',
          begin: '\\|',
          end: '\\|',
          keywords: {
            keyword: 'as'
          },
          contains: [
            TYPE_PARAM,
            TYPE_NAME,
            {
              className: 'params',
              begin: '[a-z_][a-z0-9_]*',
              relevance: 0
            }
          ]
        },

        // Type annotations in patterns
        {
          begin: ':\\s*',
          end: '(?=[,\\)\\}\\]=>]|$)',
          keywords: {
            built_in: 'Array Map Result Option Error String Number Int Float Bool DateTime Duration Any'
          },
          contains: [
            TYPE_PARAM,
            TYPE_NAME,
            {
              className: 'type',
              begin: '\\b(i32|z32|i64|z64|u32|v32|u64|v64|f32|f64|bool|string|null|error|array|datetime|duration|decimal)\\b'
            }
          ],
          relevance: 0
        }
      ]
    };
  }

  // Register the language with highlight.js
  // This script is loaded via additional-js, so hljs is already available
  console.log('Graphix highlighter: Checking for hljs...', typeof hljs);

  if (typeof hljs !== 'undefined') {
    console.log('Graphix highlighter: hljs found, registering language');
    hljs.registerLanguage('graphix', graphix);
    hljs.registerLanguage('gx', graphix); // Also register the 'gx' alias
    console.log('Graphix highlighter: Language registered');

    // Wait a bit for book.js to finish its initial highlighting pass
    // then re-highlight all graphix code blocks with our newly registered language
    setTimeout(function() {
      var blocks = document.querySelectorAll('code.language-graphix, code.language-gx');
      console.log('Graphix highlighter: Found ' + blocks.length + ' code blocks to highlight');

      blocks.forEach(function(block) {
        // Clear any existing highlighting
        block.removeAttribute('data-highlighted');
        block.classList.remove('hljs');
        block.innerHTML = block.textContent; // Reset to plain text

        // Apply our highlighting
        hljs.highlightElement(block);
        console.log('Highlighted block:', block);
      });
    }, 100);
  } else {
    console.error('Graphix highlighter: hljs not found!');
  }

  // Export for use in Node.js/CommonJS environments
  if (typeof module !== 'undefined' && module.exports) {
    module.exports = graphix;
  }

  console.log('Graphix highlighter: IIFE completed');
})();
