/**
 * WebSocket Connector
 *
 * Handles WebSocket connection and messaging
 */
import WebSocket from 'ws';

class WebSocketConnector {
    /**
     * Create a new WebSocketConnector instance
     *
     * @param {Object} options - Configuration options
     */
    constructor(options) {
        this.options = options;
        this.socket = null;
        this.channels = {};
        this.eventCallbacks = {};
        this.errorCallbacks = [];

        this.connect();
    }

    /**
     * Connect to the WebSocket server
     */
    connect() {
        const protocol = this.options.encrypted !== false ? 'wss' : 'ws';
        const url = `${protocol}://${this.options.host}${this.options.path}`;

        this.socket = new WebSocket(url);

        this.socket.on('open', () => {
            console.log('WebSocket connected');
        });

        this.socket.on('message', (data) => {
            this.handleMessage(data);
        });

        this.socket.on('error', (error) => {
            console.error('WebSocket error:', error);
            this.errorCallbacks.forEach(callback => callback(error));
        });

        this.socket.on('close', () => {
            console.log('WebSocket disconnected');
            this.reconnect();
        });
    }

    /**
     * Reconnect to the WebSocket server
     */
    reconnect() {
        setTimeout(() => {
            console.log('Reconnecting...');
            this.connect();

            // Resubscribe to all channels
            Object.keys(this.channels).forEach(channel => {
                this.subscribe(channel);
            });
        }, 1000);
    }

    /**
     * Handle incoming messages
     *
     * @param {string} data - Raw message data
     */
    handleMessage(data) {
        try {
            const message = JSON.parse(data.toString());
            const { channel, event, data: eventData } = message;

            if (this.eventCallbacks[channel] && this.eventCallbacks[channel][event]) {
                this.eventCallbacks[channel][event].forEach(callback => {
                    callback(eventData);
                });
            }
        } catch (error) {
            console.error('Error handling message:', error);
        }
    }

    /**
     * Subscribe to a channel
     *
     * @param {string} channel - Channel name
     */
    subscribe(channel) {
        if (!this.channels[channel]) {
            this.channels[channel] = true;
        }

        this.send({
            type: 'subscribe',
            channel: channel
        });
    }

    /**
     * Unsubscribe from a channel
     *
     * @param {string} channel - Channel name
     */
    unsubscribe(channel) {
        if (this.channels[channel]) {
            delete this.channels[channel];
        }

        if (this.eventCallbacks[channel]) {
            delete this.eventCallbacks[channel];
        }

        this.send({
            type: 'unsubscribe',
            channel: channel
        });
    }

    /**
     * Authorize a private/presence channel
     *
     * @param {string} channel - Channel name
     * @returns {Promise}
     */
    async authorize(channel) {
        const authEndpoint = this.options.authEndpoint || '/broadcasting/auth';

        // If authorizer function is provided, use it
        if (this.options.authorizer) {
            return this.options.authorizer(channel);
        }

        // Otherwise make a POST request to the auth endpoint
        const response = await fetch(authEndpoint, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Accept': 'application/json',
                ...this.options.auth?.headers
            },
            body: JSON.stringify({
                channel_name: channel
            })
        });

        if (!response.ok) {
            throw new Error(`Authorization failed for channel ${channel}`);
        }

        const data = await response.json();
        return data;
    }

    /**
     * Listen for an event on a channel
     *
     * @param {string} channel - Channel name
     * @param {string} event - Event name
     * @param {Function} callback - Event handler
     */
    on(channel, event, callback) {
        if (!this.eventCallbacks[channel]) {
            this.eventCallbacks[channel] = {};
        }

        if (!this.eventCallbacks[channel][event]) {
            this.eventCallbacks[channel][event] = [];
        }

        this.eventCallbacks[channel][event].push(callback);
    }

    /**
     * Remove event listener
     *
     * @param {string} channel - Channel name
     * @param {string} event - Event name
     * @param {Function} callback - Optional specific callback to remove
     */
    removeListener(channel, event, callback) {
        if (!this.eventCallbacks[channel] || !this.eventCallbacks[channel][event]) {
            return;
        }

        if (callback) {
            const index = this.eventCallbacks[channel][event].indexOf(callback);
            if (index > -1) {
                this.eventCallbacks[channel][event].splice(index, 1);
            }
        } else {
            this.eventCallbacks[channel][event] = [];
        }
    }

    /**
     * Send a whisper event
     *
     * @param {string} channel - Channel name
     * @param {string} event - Event name
     * @param {Object} data - Event data
     */
    whisper(channel, event, data) {
        this.send({
            type: 'whisper',
            channel: channel,
            event: event,
            data: data
        });
    }

    /**
     * Register error handler
     *
     * @param {Function} callback - Error handler
     */
    onError(callback) {
        this.errorCallbacks.push(callback);
    }

    /**
     * Send a message to the server
     *
     * @param {Object} data - Message data
     */
    send(data) {
        if (this.socket && this.socket.readyState === WebSocket.OPEN) {
            this.socket.send(JSON.stringify(data));
        }
    }

    /**
     * Disconnect from the server
     */
    disconnect() {
        if (this.socket) {
            this.socket.close();
            this.socket = null;
        }
    }
}

export default WebSocketConnector;
