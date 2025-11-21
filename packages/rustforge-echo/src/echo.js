/**
 * RustForge Echo - JavaScript WebSocket Client
 *
 * Laravel Echo-inspired client for RustForge WebSocket broadcasting
 */

import WebSocketConnector from './connector/websocket.js';
import Channel from './channel.js';
import PrivateChannel from './private-channel.js';
import PresenceChannel from './presence-channel.js';

class Echo {
    /**
     * Create a new Echo instance
     *
     * @param {Object} options - Configuration options
     * @param {string} options.broadcaster - Broadcaster type ('websocket')
     * @param {string} options.host - WebSocket server host
     * @param {string} options.path - WebSocket path (default: '/ws')
     * @param {string} options.authEndpoint - Auth endpoint (default: '/broadcasting/auth')
     * @param {Object} options.auth - Authentication configuration
     */
    constructor(options) {
        this.options = {
            broadcaster: 'websocket',
            host: 'localhost:8000',
            path: '/ws',
            authEndpoint: '/broadcasting/auth',
            ...options
        };

        this.channels = {};
        this.connector = this.createConnector();
    }

    /**
     * Create the appropriate connector based on configuration
     *
     * @returns {WebSocketConnector}
     */
    createConnector() {
        if (this.options.broadcaster === 'websocket') {
            return new WebSocketConnector(this.options);
        }

        throw new Error(`Broadcaster ${this.options.broadcaster} is not supported.`);
    }

    /**
     * Listen to a public channel
     *
     * @param {string} channelName - Channel name
     * @returns {Channel}
     */
    channel(channelName) {
        if (!this.channels[channelName]) {
            this.channels[channelName] = new Channel(
                this.connector,
                channelName
            );
        }

        return this.channels[channelName];
    }

    /**
     * Listen to a private channel
     *
     * @param {string} channelName - Channel name (without 'private-' prefix)
     * @returns {PrivateChannel}
     */
    private(channelName) {
        const name = `private-${channelName}`;

        if (!this.channels[name]) {
            this.channels[name] = new PrivateChannel(
                this.connector,
                name
            );
        }

        return this.channels[name];
    }

    /**
     * Listen to a presence channel
     *
     * @param {string} channelName - Channel name
     * @returns {PresenceChannel}
     */
    join(channelName) {
        const name = `presence-${channelName}`;

        if (!this.channels[name]) {
            this.channels[name] = new PresenceChannel(
                this.connector,
                name
            );
        }

        return this.channels[name];
    }

    /**
     * Leave a channel
     *
     * @param {string} channelName - Channel name
     */
    leave(channelName) {
        const channels = [channelName, `private-${channelName}`, `presence-${channelName}`];

        channels.forEach(name => {
            if (this.channels[name]) {
                this.channels[name].unsubscribe();
                delete this.channels[name];
            }
        });
    }

    /**
     * Disconnect from the server
     */
    disconnect() {
        Object.keys(this.channels).forEach(channel => {
            this.channels[channel].unsubscribe();
        });

        this.connector.disconnect();
        this.channels = {};
    }
}

export default Echo;
