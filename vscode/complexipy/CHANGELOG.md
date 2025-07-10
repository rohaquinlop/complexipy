# Change Log

## [0.1.0] - 2025-07-09

### Added
- **Status Bar Integration**:
  - Real-time file complexity overview in the status bar
  - Shows total functions and high-complexity function count
  - Visual indicators (green function icon, yellow warning, red error) based on complexity levels
  - Detailed tooltip with complexity grade (A-D), statistics, and recommendations
  - Clickable status bar item to trigger manual analysis
  - Automatic updates when switching files or making changes
  - Hides when non-Python files are active

- **Advanced Hover Message System**: Comprehensive complexity insights with rich markdown formatting
  - **Function-level hover messages**: Detailed analysis including function location, size, complexity breakdown, and context-aware refactoring strategies
  - **Line-level hover messages**: Granular complexity analysis with parent function context, complexity source identification, and targeted improvement suggestions
  - **Intelligent complexity categorization**: Automatic detection of accumulative vs. high complexity patterns with tailored recommendations
  - **Educational hover content**: Comprehensive explanations of cognitive complexity calculations, formulas, and optimization strategies
  - **Trusted hover content**: Enabled link support and interactive elements within hover messages

- **Technical complexity metrics** in hover messages:
  - Control flow structures count
  - Boolean operators count
  - Deeply nested lines detection (>2 levels)
  - High-complexity lines identification
  - Average complexity per line calculation

- **Specialized refactoring strategies** based on complexity patterns:
  - Function decomposition for accumulative complexity
  - Priority-based refactoring for high complexity functions
  - Sequential refactoring guidance (3-5 lines at a time)
  - Design pattern suggestions (Strategy, State, Command)

- **Comprehensive educational content** about cognitive complexity:
  - Base cost calculation (+1 for if/for/while/try/except)
  - Nesting penalty explanation (+1 per level)
  - Boolean operations impact (+1 per and/or)
  - Structural complexity factors
  - Complete formula breakdown

### Enhanced
- **Comprehensive hover messages**: Significantly improved hover information for both functions and lines
- **Intelligent complexity analysis**: Enhanced function complexity hover with detailed breakdown including:
  - Control flow structures count
  - Boolean operators count
  - Deeply nested lines detection
  - High-complexity lines identification
- **Smart refactoring recommendations**: Context-aware suggestions based on complexity patterns:
  - Accumulative complexity detection with targeted strategies
  - High complexity urgent refactoring priorities
  - Function decomposition and extraction guidance
- **Detailed line-level insights**: Enhanced line complexity hover with:
  - Complexity source identification (branching, loops, exception handling, etc.)
  - Complexity breakdown and calculation explanation
  - Priority fixes and specific suggestions per complexity type
  - Parent function context information
- **Educational content**: Added comprehensive explanations about cognitive complexity:
  - Formula breakdown (Base + Nesting + Boolean ops + Structural elements)
  - Key factors and penalties explanation
  - Focus strategies for optimization

### Improved
- **Visual clarity**: Better formatting and structure of hover messages
- **Actionable guidance**: More specific and prioritized refactoring recommendations
- **Context awareness**: Hover messages now adapt based on complexity patterns and nesting levels

## [0.0.3] - 2025-07-05

- Threshold adjustment

## [0.0.2] - 2025-06-12

### Added
- Updated branding assets for the extension

## [0.0.1] - 2025-06-09

### Added
- Initial release of complexipy VSCode extension
- Real-time cognitive complexity analysis for Python code
- Visual complexity indicators:
  - Function complexity shown with `ƒ` symbol
  - Line-level complexity shown with `+` symbol
  - Color-coded indicators (green for low complexity, red for high complexity)
- Automatic updates on:
  - File save
  - Active editor change
  - Text changes
- Manual analysis trigger via command palette
- Support for Python files
- Complexity thresholds:
  - Function complexity: Low (≤ 15), High (> 15)
  - Line complexity: Low (≤ 5), High (> 5)