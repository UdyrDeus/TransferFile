const form = document.getElementById('uploadForm');
const notification = document.getElementById('notification');

form.addEventListener('submit', async (e) => {
  e.preventDefault();

  notification.style.display = 'none';
  notification.className = 'notification';

  const formData = new FormData(form);

  try {
    const response = await fetch('/upload', {
      method: 'POST',
      body: formData
    });

    const text = await response.text();

    if (response.ok) {
      notification.textContent = text;
      notification.classList.add('success', 'show');
      form.reset();
    } else {
      notification.textContent = `Erreur: ${text}`;
      notification.classList.add('error', 'show');
    }
  } catch (err) {
    notification.textContent = 'Erreur réseau, veuillez réessayer.';
    notification.classList.add('error', 'show');
  }
});
