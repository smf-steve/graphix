// Tab switching functionality
document.addEventListener('DOMContentLoaded', () => {
    // Tab buttons — scoped by data-tab-group
    document.querySelectorAll('.tab-button').forEach(button => {
        button.addEventListener('click', () => {
            const tabId = button.getAttribute('data-tab');
            const group = button.closest('[data-tab-group]');
            const groupName = group ? group.getAttribute('data-tab-group') : null;

            // Find sibling buttons and panes within the same group
            const siblingButtons = group
                ? group.querySelectorAll('.tab-button')
                : document.querySelectorAll('.tab-button');
            const siblingPanes = groupName
                ? document.querySelectorAll(`.tab-pane[data-tab-group="${groupName}"]`)
                : document.querySelectorAll('.tab-pane');

            siblingButtons.forEach(btn => btn.classList.remove('active'));
            siblingPanes.forEach(pane => pane.classList.remove('active'));

            button.classList.add('active');
            document.getElementById(tabId).classList.add('active');
        });
    });

    // Mark showcase images as loaded for CSS transitions
    document.querySelectorAll('.showcase-item img').forEach(img => {
        if (img.complete) {
            img.classList.add('loaded');
        } else {
            img.addEventListener('load', () => img.classList.add('loaded'));
        }
    });

    // Scroll animation for sections
    const animatedElements = document.querySelectorAll('.feature-card, .showcase-item');

    const scrollObserver = new IntersectionObserver((entries) => {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                entry.target.style.opacity = '1';
                entry.target.style.transform = 'translateY(0)';
            }
        });
    }, {
        threshold: 0.1
    });

    animatedElements.forEach(el => {
        el.style.opacity = '0';
        el.style.transform = 'translateY(20px)';
        el.style.transition = 'opacity 0.6s ease-out, transform 0.6s ease-out';
        scrollObserver.observe(el);
    });

    // Smooth scroll for anchor links
    document.querySelectorAll('a[href^="#"]').forEach(anchor => {
        anchor.addEventListener('click', function (e) {
            e.preventDefault();
            const target = document.querySelector(this.getAttribute('href'));

            if (target) {
                target.scrollIntoView({
                    behavior: 'smooth',
                    block: 'start'
                });
            }
        });
    });

    // Keyboard navigation for tabs — scoped per group
    document.querySelectorAll('[data-tab-group]').forEach(group => {
        const buttons = Array.from(group.querySelectorAll('.tab-button'));
        buttons.forEach((button, index) => {
            button.addEventListener('keydown', (e) => {
                let targetButton = null;

                if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
                    e.preventDefault();
                    targetButton = buttons[index + 1] || buttons[0];
                } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
                    e.preventDefault();
                    targetButton = buttons[index - 1] || buttons[buttons.length - 1];
                }

                if (targetButton) {
                    targetButton.focus();
                    targetButton.click();
                }
            });
        });
    });
});
