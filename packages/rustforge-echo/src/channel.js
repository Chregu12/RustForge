/**
 * Public Channel
 *
 * Represents a public broadcast channel
 */
class Channel {
    /**
     * Create a new Channel instance
     *
     * @param {Object} connector - WebSocket connector
     * @param {string} name - Channel name
     */
    constructor(connector, name) {
        this.connector = connector;
        this.name = name;
        this.listeners = {};
        this.subscribed = false;

        this.subscribe();
    }

    /**
     * Subscribe to the channel
     */
    subscribe() {
        this.connector.subscribe(this.name);
        this.subscribed = true;
    }

    /**
     * Unsubscribe from the channel
     */
    unsubscribe() {
        this.connector.unsubscribe(this.name);
        this.subscribed = false;
        this.listeners = {};
    }

    /**
     * Listen for an event on the channel
     *
     * @param {string} event - Event name
     * @param {Function} callback - Event handler
     * @returns {Channel}
     */
    listen(event, callback) {
        if (!this.listeners[event]) {
            this.listeners[event] = [];
        }

        this.listeners[event].push(callback);

        this.connector.on(this.name, event, callback);

        return this;
    }

    /**
     * Listen for a whisper event
     *
     * @param {string} event - Event name
     * @param {Function} callback - Event handler
     * @returns {Channel}
     */
    listenForWhisper(event, callback) {
        return this.listen(`.whisper:${event}`, callback);
    }

    /**
     * Send a whisper event
     *
     * @param {string} event - Event name
     * @param {Object} data - Event data
     * @returns {Channel}
     */
    whisper(event, data) {
        this.connector.whisper(this.name, event, data);
        return this;
    }

    /**
     * Stop listening for an event
     *
     * @param {string} event - Event name
     * @param {Function} callback - Optional specific callback to remove
     * @returns {Channel}
     */
    stopListening(event, callback) {
        if (!this.listeners[event]) {
            return this;
        }

        if (callback) {
            const index = this.listeners[event].indexOf(callback);
            if (index > -1) {
                this.listeners[event].splice(index, 1);
            }
        } else {
            this.listeners[event] = [];
        }

        this.connector.removeListener(this.name, event, callback);

        return this;
    }

    /**
     * Trigger an error handler
     *
     * @param {Function} callback - Error handler
     * @returns {Channel}
     */
    error(callback) {
        this.connector.onError(callback);
        return this;
    }
}

export default Channel;
