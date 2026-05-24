const CHARS = '!@#$%^&*()_+-=[]{}|;:,.<>?/~`QWERTYUIOPASDFGHJKLZXCVBNMqwertyuiopasdfghjklzxcvbnm0123456789';

function randomChar() {
    return CHARS[Math.floor(Math.random() * CHARS.length)];
}

function scrambleTo(el, targetText, duration = 500, onDone) {
    const steps = 20;
    const interval = duration / steps;
    let frame = 0;

    if (el._scrambleInterval) {
        clearInterval(el._scrambleInterval);
        el._scrambleInterval = null;
    }

    el._isScrambling = false;

    // Фиксируем ширину только если элемент не имеет фиксированной ширины от CSS
    // и не является блочным элементом на всю ширину
    const computedStyle = window.getComputedStyle(el);
    const isFullWidth = computedStyle.display === 'flex' && computedStyle.width === '100%';

    if (!el.style.width && !el.getAttribute('data-width-fixed') && !isFullWidth) {
        const tempSpan = document.createElement('span');
        tempSpan.style.position = 'absolute';
        tempSpan.style.visibility = 'hidden';
        tempSpan.style.whiteSpace = 'nowrap';
        tempSpan.style.font = computedStyle.font;
        tempSpan.style.fontSize = computedStyle.fontSize;
        tempSpan.style.fontFamily = computedStyle.fontFamily;
        tempSpan.style.fontWeight = computedStyle.fontWeight;
        tempSpan.style.letterSpacing = computedStyle.letterSpacing;
        tempSpan.textContent = targetText;
        document.body.appendChild(tempSpan);
        const finalWidth = tempSpan.offsetWidth;
        document.body.removeChild(tempSpan);

        if (finalWidth > 0 && finalWidth < 500) { // Не фиксируем для огромных элементов
            el.style.minWidth = finalWidth + 'px';
            el.style.display = 'inline-flex'; // Для кнопок с flex
            el.setAttribute('data-width-fixed', 'true');
        }
    }

    el._isScrambling = true;

    el._scrambleInterval = setInterval(() => {
        if (!el.isConnected || !el._isScrambling) {
            if (el._scrambleInterval) {
                clearInterval(el._scrambleInterval);
                el._scrambleInterval = null;
            }
            return;
        }

        const progress = frame / steps;
        const revealedCount = Math.floor(progress * targetText.length);

        let result = '';
        for (let i = 0; i < targetText.length; i++) {
            if (targetText[i] === ' ') result += ' ';
            else if (i < revealedCount) result += targetText[i];
            else result += randomChar();
        }

        el.textContent = result;
        frame++;

        if (frame > steps) {
            clearInterval(el._scrambleInterval);
            el._scrambleInterval = null;
            el._isScrambling = false;
            el.textContent = targetText;

            if (onDone) onDone();
        }
    }, interval);
}

document.addEventListener('DOMContentLoaded', () => {
    const navLinks = document.querySelectorAll('.nav-link');
    const buttons = document.querySelectorAll('.btn');

    const animatedElements = [...navLinks, ...buttons];

    animatedElements.forEach(el => {
        const originalText = el.textContent.trim();
        el.setAttribute('data-original-text', originalText);

        el.addEventListener('mouseenter', () => {
            if (el._isScrambling) return;

            el.classList.add('scrambling');

            clearInterval(el._scrambleInterval);
            el.textContent = originalText;
            scrambleTo(el, originalText, 500);
        });

        el.addEventListener('mouseleave', () => {
            if (el._scrambleInterval) {
                clearInterval(el._scrambleInterval);
                el._scrambleInterval = null;
            }

            el._isScrambling = false;
            const text = el.getAttribute('data-original-text') || originalText;
            el.textContent = text;
            el.classList.remove('scrambling');
        });
    });

    // Обработка кликов для немедленного перехода
    document.querySelectorAll('.nav-link, .btn').forEach(el => {
        el.addEventListener('click', () => {
            if (el._scrambleInterval) {
                clearInterval(el._scrambleInterval);
                el._scrambleInterval = null;
                el._isScrambling = false;
                el.textContent = el.getAttribute('data-original-text') || el.textContent;
            }
        });
    });
});