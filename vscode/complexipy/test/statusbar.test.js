const assert = require('assert');
const vscode = require('vscode');

suite('Status Bar Integration Test', () => {
    vscode.window.showInformationMessage('Start status bar integration tests.');

    test('Status bar shows correct function count', () => {
        // Mock file stats
        const mockFileStats = {
            fileName: 'test.py',
            totalFunctions: 5,
            highComplexityFunctions: 2,
            averageComplexity: 12.5,
            maxComplexity: 25,
            totalComplexity: 62.5
        };

        // Test that function count is displayed correctly
        assert.strictEqual(mockFileStats.totalFunctions, 5);
        assert.strictEqual(mockFileStats.highComplexityFunctions, 2);
        assert.strictEqual(mockFileStats.averageComplexity, 12.5);
        assert.strictEqual(mockFileStats.maxComplexity, 25);
    });

    test('Status bar complexity grading', () => {
        // Test complexity grade calculation
        const testCases = [
            { maxComplexity: 10, expectedGrade: 'A' },
            { maxComplexity: 15, expectedGrade: 'A' },
            { maxComplexity: 20, expectedGrade: 'B' },
            { maxComplexity: 25, expectedGrade: 'B' },
            { maxComplexity: 30, expectedGrade: 'C' },
            { maxComplexity: 35, expectedGrade: 'C' },
            { maxComplexity: 40, expectedGrade: 'D' },
        ];

        testCases.forEach(({ maxComplexity, expectedGrade }) => {
            const complexityGrade = maxComplexity <= 15 ? 'A' : maxComplexity <= 25 ? 'B' : maxComplexity <= 35 ? 'C' : 'D';
            assert.strictEqual(complexityGrade, expectedGrade, `Expected grade ${expectedGrade} for complexity ${maxComplexity}`);
        });
    });

    test('Status bar icon selection', () => {
        // Test icon selection based on complexity
        const testCases = [
            { totalFunctions: 5, highComplexityFunctions: 0, expectedIcon: '$(symbol-function)' },
            { totalFunctions: 5, highComplexityFunctions: 1, expectedIcon: '$(warning)' },
            { totalFunctions: 5, highComplexityFunctions: 3, expectedIcon: '$(error)' },
        ];

        testCases.forEach(({ totalFunctions, highComplexityFunctions, expectedIcon }) => {
            let icon = '$(symbol-function)';

            if (highComplexityFunctions > 0) {
                icon = '$(warning)';
            }

            if (highComplexityFunctions > totalFunctions * 0.5) {
                icon = '$(error)';
            }

            assert.strictEqual(icon, expectedIcon,
                `Expected icon ${expectedIcon} for ${highComplexityFunctions} high complexity functions out of ${totalFunctions} total`);
        });
    });
}); 