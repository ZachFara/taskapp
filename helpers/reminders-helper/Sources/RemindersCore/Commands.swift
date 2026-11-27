import Foundation

public enum Due: Equatable {
    case date(String)
    case datetime(String, tz: String)
}

public func mapDue(_ c: DateComponents?) -> Due? {
    guard let c = c, let y = c.year, let m = c.month, let d = c.day else { return nil }
    if c.hour == nil { return .date(String(format: "%04d-%02d-%02d", y, m, d)) }
    var cal = Calendar(identifier: .gregorian)
    let tz = c.timeZone ?? TimeZone.current
    cal.timeZone = tz
    guard let date = cal.date(from: c) else { return nil }
    let f = ISO8601DateFormatter()
    f.timeZone = TimeZone(identifier: "UTC")
    return .datetime(f.string(from: date), tz: tz.identifier)
}

func json(_ value: Any) -> String {
    let data = try! JSONSerialization.data(withJSONObject: value, options: [.sortedKeys, .withoutEscapingSlashes])
    return String(data: data, encoding: .utf8)!
}

func reminderJSON(_ r: RawReminder) -> [String: Any] {
    var due: Any = NSNull()
    switch mapDue(r.due) {
    case .date(let d)?: due = ["date": d]
    case .datetime(let t, let tz)?: due = ["datetime": t, "tz": tz]
    case nil: break
    }
    return ["external_id": r.externalId, "title": r.title, "notes": r.notes ?? NSNull(), "due": due]
}

/// Runs one subcommand, printing a single JSON value; returns the exit code.
public func run(_ args: [String], store: ReminderStore, out: @escaping (String) -> Void) async -> Int32 {
    func fail(_ message: String) -> Int32 { out(json(["error": message])); return 1 }
    func listArg() -> String? {
        guard let i = args.firstIndex(of: "--list"), i + 1 < args.count else { return nil }
        return args[i + 1]
    }
    let needsAccess = ["lists", "fetch", "delete", "watch"]
    if let cmd = args.first, needsAccess.contains(cmd), !store.isAuthorized {
        return fail("Reminders access is off: press Authorize in Settings > Reminders")
    }
    do {
        switch args.first {
        case "authorize":
            out(json(["granted": try await store.requestAccess()]))
        case "lists":
            out(json(try store.lists().map { ["id": $0.id, "title": $0.title] }))
        case "fetch":
            guard let id = listArg() else { return fail("fetch requires --list <id>") }
            let items = try await store.reminders(inList: id).filter { !$0.isCompleted }
            out(json(items.map(reminderJSON)))
        case "delete":
            guard let list = listArg() else { return fail("delete requires --list <id>") }
            guard try store.lists().contains(where: { $0.id == list }) else { return fail("unknown list: \(list)") }
            var deleted: [String] = [], missing: [String] = []
            for id in args.dropFirst().filter({ $0 != "--list" && $0 != list }) {
                if try store.delete(externalId: id, inList: list) { deleted.append(id) } else { missing.append(id) }
            }
            out(json(["deleted": deleted, "missing": missing]))
        case "watch":
            guard let id = listArg() else { return fail("watch requires --list <id>") }
            guard try store.lists().contains(where: { $0.id == id }) else { return fail("unknown list: \(id)") }
            store.onChange { out(json(["changed": true])) }
            // The caller keeps the process alive until stdin closes.
        case let other:
            return fail("unknown subcommand: \(other ?? "(none)")")
        }
        return 0
    } catch let e as HelperError {
        return fail(e.message)
    } catch {
        return fail(error.localizedDescription)
    }
}
