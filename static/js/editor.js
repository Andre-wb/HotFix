class CodeEditor {
    constructor(textarea) {
        this.textarea = textarea;
        this.setupTabHandler();
        this.setupBracketMatching();
        this.setupIndentation();
    }

    setupTabHandler() {
        this.textarea.addEventListener('keydown', (e) => {
            if (e.key === 'Tab') {
                e.preventDefault();
                const start = this.textarea.selectionStart;
                const end = this.textarea.selectionEnd;

                this.textarea.value = this.textarea.value.substring(0, start) +
                    '    ' +
                    this.textarea.value.substring(end);
                this.textarea.selectionStart = this.textarea.selectionEnd = start + 4;
            }
        });
    }

    setupBracketMatching() {
        const pairs = { '(': ')', '{': '}', '[': ']', '"': '"', "'": "'" };

        this.textarea.addEventListener('keydown', (e) => {
            if (pairs[e.key]) {
                e.preventDefault();
                const start = this.textarea.selectionStart;

                this.textarea.value = this.textarea.value.substring(0, start) +
                    e.key + pairs[e.key] +
                    this.textarea.value.substring(start);
                this.textarea.selectionStart = this.textarea.selectionEnd = start + 1;
            }
        });
    }

    setupIndentation() {
        this.textarea.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                e.preventDefault();
                const start = this.textarea.selectionStart;
                const beforeCursor = this.textarea.value.substring(0, start);
                const lastLineStart = beforeCursor.lastIndexOf('\n') + 1;
                const currentIndent = beforeCursor.substring(lastLineStart).match(/^\s*/)[0];

                let extraIndent = '';
                if (beforeCursor.substring(lastLineStart).trim().endsWith('{')) {
                    extraIndent = '    ';
                }

                this.textarea.value = this.textarea.value.substring(0, start) +
                    '\n' + currentIndent + extraIndent +
                    this.textarea.value.substring(start);
                this.textarea.selectionStart = this.textarea.selectionEnd =
                    start + 1 + currentIndent.length + extraIndent.length;
            }
        });
    }
}

class TimerLock {
    constructor(form, timeLeft) {
        this.form = form;
        this.textarea = form.querySelector('textarea');
        this.submitBtn = form.querySelector('button');
        this.timeLeft = timeLeft;
        this.timerElement = document.getElementById('timer');

        if (this.timerElement) {
            this.startTimer();
        }
    }

    startTimer() {
        const interval = setInterval(() => {
            this.timeLeft--;
            if (this.timerElement) {
                this.timerElement.textContent = this.timeLeft;
            }

            if (this.timeLeft <= 0) {
                clearInterval(interval);
                this.lockCode();
            }
        }, 1000);
    }

    lockCode() {
        this.textarea.disabled = true;
        this.submitBtn.disabled = true;
        this.textarea.style.opacity = '0.6';
        this.textarea.value = 'Time expired! You can no longer submit.';

        if (this.timerElement) {
            this.timerElement.textContent = 'EXPIRED';
            this.timerElement.style.color = '#e74c3c';
        }

        alert("⏰ Time's up! The code has been locked.");
    }
}

document.addEventListener('DOMContentLoaded', () => {
    const textareas = document.querySelectorAll('textarea');
    textareas.forEach(textarea => new CodeEditor(textarea));

    const form = document.getElementById('solve-form');
    const timerSpan = document.getElementById('timer');
    if (form && timerSpan) {
        const timeLeft = parseInt(timerSpan.textContent);
        new TimerLock(form, timeLeft);
    }
});