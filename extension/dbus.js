import Gio from 'gi://Gio';

const DBUS_XML = `
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
  </interface>
</node>`;

const DaemonProxyClass = Gio.DBusProxy.makeProxyWrapper(DBUS_XML);

/**
 * Create a DBus proxy for the daemon.
 *
 * Returns a Promise that resolves to the proxy, or rejects if the daemon is
 * not reachable.  The promise resolves even if the daemon is not currently
 * running — method calls will simply fail until it starts.
 */
export function createDaemonProxy() {
    return new Promise((resolve, reject) => {
        try {
            new DaemonProxyClass(
                Gio.DBus.session,
                'org.transcriber.Daemon',
                '/org/transcriber/Daemon',
                (proxy, error) => {
                    if (error) reject(error);
                    else resolve(proxy);
                },
                null,
                Gio.DBusProxyFlags.DO_NOT_AUTO_START,
            );
        } catch (e) {
            reject(e);
        }
    });
}
