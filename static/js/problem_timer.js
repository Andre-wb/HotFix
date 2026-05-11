let timeLeft = {{ problem.time_limit_seconds }};
const timerEl = document.getElementById('timer');
const form = document.getElementById('solve-form');

const interval = setInterval(() => {
    timeLeft--;
    timerEl.textContent = timeLeft;
    if (timeLeft <= 0) {
        clearInterval(interval);
        form.querySelector('button').disabled = true;
        timerEl.textContent = "EXPIRED";
        alert("Time's up!");
    }
}, 1000);