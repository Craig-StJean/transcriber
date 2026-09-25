import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

export const BUS_NAME    = 'org.transcriber.Daemon';
export const OBJECT_PATH = '/org/transcriber/Daemon';
export const INTERFACE   = 'org.transcriber.Daemon';

/**
 * The subset of the daemon's interface the extension uses. Kept here as the
 * reference for the signal/method signatures handled in extension.js; the
 * extension talks to the bus directly (Gio.DBusConnection.call /
 * signal_subscribe) so that method calls can DBus-activate the daemon and
 * signals can be pinned to the daemon's unique bus name.
 */
export const INTERFACE_XML = `
<node>
  <interface name="org.transcriber.Daemon">
    <method name="StartRecording"/>
    <method name="StopRecording"/>
    <method name="Cancel"/>
    <property name="CurrentState" type="s" access="read"/>
    <signal name="StateChanged">
      <arg type="s" name="state"/>
    </signal>
    <signal name="AudioLevel">
      <arg type="d" name="level"/>
    </signal>
    <signal name="TranscriptionReady">
      <arg type="s" name="text"/>
    </signal>
    <signal name="TranscriptionChunk">
      <arg type="s" name="text"/>
    </signal>
    <signal name="ErrorOccurred">
      <arg type="s" name="message"/>
    </signal>
    <signal name="SessionDiscarded">
      <arg type="s" name="reason"/>
    </signal>
  </interface>
</node>`;

/**
 * Call a no-argument daemon method.
 *
 * Unless `autoStart` is false the call is sent without NO_AUTO_START, so the
 * bus will DBus-activate transcriber-daemon if it isn't running yet.
 *
 * @returns {Promise<GLib.Variant>}
 */
export function callDaemon(method, cancellable, { autoStart = true } = {}) {
    return new Promise((resolve, reject) => {
        Gio.DBus.session.call(
            BUS_NAME, OBJECT_PATH, INTERFACE, method,
            null, null,
            autoStart ? Gio.DBusCallFlags.NONE : Gio.DBusCallFlags.NO_AUTO_START,
            -1,
            cancellable,
            (conn, res) => {
                try {
                    resolve(conn.call_finish(res));
                } catch (e) {
                    reject(e);
                }
            });
    });
}

/**
 * Read the daemon's CurrentState property. Never auto-starts the daemon.
 *
 * @returns {Promise<string>}
 */
export function getCurrentState(cancellable) {
    return new Promise((resolve, reject) => {
        Gio.DBus.session.call(
            BUS_NAME, OBJECT_PATH, 'org.freedesktop.DBus.Properties', 'Get',
            new GLib.Variant('(ss)', [INTERFACE, 'CurrentState']),
            new GLib.VariantType('(v)'),
            Gio.DBusCallFlags.NO_AUTO_START,
            -1,
            cancellable,
            (conn, res) => {
                try {
                    const [value] = conn.call_finish(res).recursiveUnpack();
                    resolve(value);
                } catch (e) {
                    reject(e);
                }
            });
    });
}

export function isCancelledError(e) {
    return e instanceof GLib.Error &&
        e.matches(Gio.IOErrorEnum, Gio.IOErrorEnum.CANCELLED);
}
