# Complexipy VSCode Extension

A Visual Studio Code extension that provides real-time cognitive complexity analysis for Python code. This extension helps developers identify and manage code complexity by providing visual indicators for both function-level and line-level complexity.

## Features

- **Real-time Complexity Analysis**: Automatically analyzes Python code as you type
- **Visual Complexity Indicators**:
  - Function complexity shown with `ƒ` symbol
  - Line-level complexity shown with `+` symbol
  - Color-coded indicators:
    - Green: Low complexity (functions ≤ 15, lines ≤ 5)
    - Red: High complexity (functions > 15, lines > 5)
- **Status Bar Integration**:
  - Overall file complexity overview in the status bar
  - Shows total functions and high-complexity function count
  - Visual indicators (green/yellow/red) based on complexity levels
  - Detailed tooltip with complexity grade and recommendations
  - Clickable to trigger manual analysis
- **Comprehensive Hover Information**:
  - Detailed function complexity breakdown
  - Line-level complexity explanations
  - Actionable refactoring suggestions
  - Educational content about cognitive complexity
- **Automatic Updates**: Complexity analysis updates on:
  - File save
  - Active editor change
  - Text changes

## Requirements

- Visual Studio Code version 1.85.0 or higher
- Python files to analyze

## Installation

1. Open VS Code
2. Go to the Extensions view (Ctrl+Shift+X / Cmd+Shift+X)
3. Search for "complexipy"
4. Click Install

## Usage

The extension automatically activates when you open a Python file. You'll see complexity indicators appear at the end of each line:

- `ƒ N`: Function complexity score (where N is the complexity value)
- `+N`: Line-level complexity score (where N is the complexity value)

### Manual Analysis

You can trigger a manual analysis by:
1. Opening the Command Palette (Ctrl+Shift+P / Cmd+Shift+P)
2. Typing "complexipy"
3. Selecting the "complexipy" command

## Complexity Thresholds

- **Function Complexity**:
  - Low: ≤ 15
  - High: > 15

- **Line Complexity**:
  - Low: ≤ 5
  - High: > 5

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This extension is licensed under the MIT License - see the LICENSE file for details.
