// Resume Web Audio contexts after the first user gesture (Chrome/Firefox/Safari autoplay policy).
// Must load before the WASM bundle so AudioContext construction is intercepted.
// https://developer.chrome.com/blog/web-audio-autoplay
(function () {
    const audioContextList = [];
    const userInputEventNames = [
        "click",
        "contextmenu",
        "auxclick",
        "dblclick",
        "mousedown",
        "mouseup",
        "pointerup",
        "touchend",
        "keydown",
        "keyup",
    ];

    self.AudioContext = new Proxy(self.AudioContext, {
        construct(target, args) {
            const result = new target(...args);
            audioContextList.push(result);
            return result;
        },
    });

    function resumeAllContexts() {
        let running = 0;

        audioContextList.forEach((context) => {
            if (context.state !== "running") {
                context.resume();
            } else {
                running++;
            }
        });

        if (running > 0 && running === audioContextList.length) {
            userInputEventNames.forEach((eventName) => {
                document.removeEventListener(eventName, resumeAllContexts);
            });
        }
    }

    userInputEventNames.forEach((eventName) => {
        document.addEventListener(eventName, resumeAllContexts);
    });
})();
