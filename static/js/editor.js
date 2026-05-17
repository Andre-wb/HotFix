class CodeEditor {
    constructor(textarea) {
        this.textarea = textarea;
        this.lineNumbers = document.getElementById('line-numbers');
        this.cursorPosition = document.getElementById('cursor-position');

        this.unescapeTextarea();
        this.setupTabHandler();
        this.setupBracketMatching();
        this.setupIndentation();
        this.setupAutoPairRemoval();
        this.setupLineNumbers();
        this.setupCursorTracking();
        this.setupKeyboardShortcuts();
    }

    unescapeTextarea() {
        let value = this.textarea.value;
        value = value.replace(/\\n/g, '\n');
        value = value.replace(/\\t/g, '\t');
        value = value.replace(/\\"/g, '"');
        value = value.replace(/\\'/g, "'");
        value = value.replace(/\\\\/g, '\\');
        this.textarea.value = value;
    }

    setupTabHandler() {
        this.textarea.addEventListener('keydown', (e) => {
            if (e.key === 'Tab') {
                e.preventDefault();
                const start = this.textarea.selectionStart;
                const end = this.textarea.selectionEnd;

                // If text is selected, indent all selected lines
                if (start !== end) {
                    const selectedText = this.textarea.value.substring(start, end);
                    const lines = selectedText.split('\n');
                    const indentedLines = lines.map(line => '    ' + line);
                    const newText = indentedLines.join('\n');

                    this.textarea.value =
                        this.textarea.value.substring(0, start) +
                        newText +
                        this.textarea.value.substring(end);

                    this.textarea.selectionStart = start;
                    this.textarea.selectionEnd = start + newText.length;
                } else {
                    // Insert 4 spaces
                    this.textarea.value =
                        this.textarea.value.substring(0, start) +
                        '    ' +
                        this.textarea.value.substring(end);
                    this.textarea.selectionStart = this.textarea.selectionEnd = start + 4;
                }

                this.updateLineNumbers();
            }
        });
    }

    setupBracketMatching() {
        const pairs = { '(': ')', '{': '}', '[': ']', '"': '"', "'": "'" };

        this.textarea.addEventListener('keydown', (e) => {
            if (pairs[e.key]) {
                e.preventDefault();
                const start = this.textarea.selectionStart;
                const end = this.textarea.selectionEnd;
                const text = this.textarea.value;

                if (start !== end) {
                    const selectedText = text.substring(start, end);
                    this.textarea.value =
                        text.substring(0, start) +
                        e.key + selectedText + pairs[e.key] +
                        text.substring(end);
                    this.textarea.selectionStart = start + 1;
                    this.textarea.selectionEnd = end + 1;
                } else {
                    this.textarea.value =
                        text.substring(0, start) +
                        e.key + pairs[e.key] +
                        text.substring(start);
                    this.textarea.selectionStart = this.textarea.selectionEnd = start + 1;
                }
            }

            // Smart backspace for pairs
            if (e.key === 'Backspace') {
                const start = this.textarea.selectionStart;
                const end = this.textarea.selectionEnd;
                const text = this.textarea.value;

                if (start === end && start > 0) {
                    const leftChar = text[start - 1];
                    const rightChar = text[start];

                    if ((leftChar === '(' && rightChar === ')') ||
                        (leftChar === '{' && rightChar === '}') ||
                        (leftChar === '[' && rightChar === ']') ||
                        (leftChar === '"' && rightChar === '"') ||
                        (leftChar === "'" && rightChar === "'")) {
                        e.preventDefault();
                        this.textarea.value = text.substring(0, start - 1) + text.substring(start + 1);
                        this.textarea.selectionStart = this.textarea.selectionEnd = start - 1;
                    }
                }
            }
        });
    }

    setupIndentation() {
        this.textarea.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                const start = this.textarea.selectionStart;
                const text = this.textarea.value;

                let lineStart = text.lastIndexOf('\n', start - 1);
                if (lineStart === -1) lineStart = 0;
                else lineStart += 1;

                const currentLine = text.substring(lineStart, start);
                const indentMatch = currentLine.match(/^\s*/);
                const currentIndent = indentMatch ? indentMatch[0] : '';

                let extraIndent = '';
                const trimmedLine = currentLine.trim();
                if (trimmedLine.endsWith('{') || trimmedLine.endsWith('(') || trimmedLine.endsWith('[')) {
                    extraIndent = '    ';
                }

                let closingBrace = false;
                const nextChars = text.substring(start, start + 1);
                if (nextChars === '}' || nextChars === ')' || nextChars === ']') {
                    closingBrace = true;
                }

                let newIndent = currentIndent;
                if (closingBrace && currentIndent.length >= 4) {
                    newIndent = currentIndent.substring(0, currentIndent.length - 4);
                }

                const insertText = '\n' + newIndent + extraIndent;
                const cursorPos = start + insertText.length;

                this.textarea.value =
                    text.substring(0, start) +
                    insertText +
                    (closingBrace ? '\n' + newIndent : '') +
                    text.substring(start);

                this.textarea.selectionStart = this.textarea.selectionEnd = cursorPos;
                this.updateLineNumbers();
            }
        });
    }

    setupAutoPairRemoval() {
        const pairs = { ')': '(', '}': '{', ']': '[', '"': '"', "'": "'" };

        this.textarea.addEventListener('keydown', (e) => {
            if (pairs[e.key]) {
                const start = this.textarea.selectionStart;
                const end = this.textarea.selectionEnd;
                const text = this.textarea.value;

                if (start === end && start < text.length && text[start] === e.key) {
                    const leftChar = text[start - 1];
                    if (leftChar === pairs[e.key]) {
                        e.preventDefault();
                        this.textarea.selectionStart = this.textarea.selectionEnd = start + 1;
                    }
                }
            }
        });
    }

    setupLineNumbers() {
        if (!this.lineNumbers) return;

        const updateLines = () => {
            const lines = this.textarea.value.split('\n').length;
            this.lineNumbers.innerHTML = Array.from({ length: lines }, (_, i) => i + 1).join('<br>');
        };

        this.textarea.addEventListener('input', updateLines);
        this.textarea.addEventListener('scroll', () => {
            this.lineNumbers.scrollTop = this.textarea.scrollTop;
        });

        updateLines();
    }

    setupCursorTracking() {
        if (!this.cursorPosition) return;

        const updateCursor = () => {
            const text = this.textarea.value.substring(0, this.textarea.selectionStart);
            const lines = text.split('\n');
            const line = lines.length;
            const col = lines[lines.length - 1].length + 1;
            this.cursorPosition.textContent = `Ln ${line}, Col ${col}`;
        };

        this.textarea.addEventListener('keyup', updateCursor);
        this.textarea.addEventListener('click', updateCursor);
        updateCursor();
    }

    setupKeyboardShortcuts() {
        document.addEventListener('keydown', (e) => {
            // Ctrl+Enter to submit
            if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
                const form = document.getElementById('solve-form');
                if (form) {
                    e.preventDefault();
                    const submitBtn = form.querySelector('button[type="submit"]');
                    if (submitBtn && !submitBtn.disabled) {
                        submitBtn.click();
                    }
                }
            }

            // Ctrl+S to save to localStorage
            if ((e.ctrlKey || e.metaKey) && e.key === 's') {
                e.preventDefault();
                if (this.textarea.id === 'problem-code') {
                    localStorage.setItem('saved_code', this.textarea.value);
                    showNotification('💾 Code saved to local storage', 'success');
                }
            }
        });
    }

    updateLineNumbers() {
        if (this.lineNumbers) {
            const lines = this.textarea.value.split('\n').length;
            this.lineNumbers.innerHTML = Array.from({ length: lines }, (_, i) => i + 1).join('<br>');
        }
    }
}

class TimerLock {
    constructor(form, timeLimit) {
        this.form = form;
        this.textarea = form.querySelector('textarea');
        this.submitBtn = form.querySelector('button[type="submit"]');
        this.timeLeft = timeLimit;
        this.timerElement = document.getElementById('timer-value');
        this.timerBadge = document.getElementById('timer');
        this.interval = null;

        if (this.timerElement) {
            this.startTimer();
        }
    }

    startTimer() {
        this.updateDisplay();
        this.interval = setInterval(() => {
            this.timeLeft--;
            this.updateDisplay();

            if (this.timeLeft <= 0) {
                this.lockCode();
            }
        }, 1000);
    }

    updateDisplay() {
        if (!this.timerElement) return;

        const minutes = Math.floor(this.timeLeft / 60);
        const seconds = this.timeLeft % 60;

        this.timerElement.textContent = minutes > 0
            ? `${minutes}:${seconds.toString().padStart(2, '0')}`
            : `${seconds}s`;

        // Add warning styles
        if (this.timeLeft <= 30 && this.timerBadge) {
            this.timerBadge.classList.add('warning');
        }
    }

    lockCode() {
        if (this.interval) {
            clearInterval(this.interval);
            this.interval = null;
        }

        if (this.textarea) {
            this.textarea.disabled = true;
            this.textarea.style.opacity = '0.5';
            this.textarea.style.cursor = 'not-allowed';
        }

        if (this.submitBtn) {
            this.submitBtn.disabled = true;
            this.submitBtn.style.opacity = '0.5';
            this.submitBtn.style.cursor = 'not-allowed';
        }

        if (this.timerElement) {
            this.timerElement.textContent = 'EXPIRED';
        }

        if (this.timerBadge) {
            this.timerBadge.style.borderColor = 'var(--error)';
            this.timerBadge.style.color = 'var(--error)';
        }

        showNotification('⏰ Time\'s up! The code editor has been locked.', 'error');
    }
}

function showNotification(message, type = 'info') {
    const existing = document.querySelector('.notification-toast');
    if (existing) existing.remove();

    const notification = document.createElement('div');
    notification.className = 'notification-toast';

    const colors = {
        success: 'var(--success)',
        error: 'var(--error)',
        info: 'var(--accent)'
    };

    notification.style.cssText = `
        position: fixed;
        top: 80px;
        right: 20px;
        background: var(--bg-card);
        color: ${colors[type] || colors.info};
        padding: 12px 20px;
        border-radius: var(--radius-md);
        font-weight: 600;
        z-index: 1000;
        border: 1px solid var(--border);
        box-shadow: var(--shadow-lg);
        animation: slideIn 0.3s ease;
        font-size: 0.9rem;
        display: flex;
        align-items: center;
        gap: 8px;
    `;

    notification.textContent = message;
    document.body.appendChild(notification);

    setTimeout(() => {
        notification.style.animation = 'fadeOut 0.3s ease';
        setTimeout(() => notification.remove(), 300);
    }, 3000);
}

// Add fadeOut animation
const fadeOutStyle = document.createElement('style');
fadeOutStyle.textContent = `
    @keyframes fadeOut {
        from { opacity: 1; transform: translateX(0); }
        to { opacity: 0; transform: translateX(20px); }
    }
`;
document.head.appendChild(fadeOutStyle);

// Initialize on DOM ready
document.addEventListener('DOMContentLoaded', () => {
    // Initialize all textareas as editors
    const textareas = document.querySelectorAll('textarea');
    textareas.forEach(textarea => {
        if (textarea.classList.contains('code-textarea') || textarea.closest('.editor-container')) {
            new CodeEditor(textarea);
        }
    });

    // Initialize timer if present
    const form = document.getElementById('solve-form');
    const timerSpan = document.getElementById('timer-value');

    if (form && timerSpan && window.problemTimeLimit) {
        const timeLeft = parseInt(timerSpan.textContent) || window.problemTimeLimit;
        if (!isNaN(timeLeft) && timeLeft > 0) {
            new TimerLock(form, timeLeft);
        }
    }

    // Restore saved code
    const problemCode = document.getElementById('problem-code');
    if (problemCode && localStorage.getItem('saved_code')) {
        const saved = localStorage.getItem('saved_code');
        if (saved && confirm('You have unsaved code from a previous session. Would you like to restore it?')) {
            problemCode.value = saved;
            localStorage.removeItem('saved_code');
            // Trigger input event to update line numbers
            problemCode.dispatchEvent(new Event('input'));
        }
    }
});