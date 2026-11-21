import Foundation
import Testing
@testable import RemindersCore

final class FakeStore: ReminderStore {
    var items: [String: [RawReminder]] = ["L1": []]
    var handler: (() -> Void)?
    func requestAccess() async throws -> Bool { true }
    func lists() throws -> [ReminderList] { items.keys.sorted().map { ReminderList(id: $0, title: "T-\($0)") } }
    func reminders(inList id: String) async throws -> [RawReminder] {
        guard let r = items[id] else { throw HelperError("unknown list: \(id)") }
        return r
    }
    func delete(externalId: String, inList id: String) throws -> Bool {
        guard let i = items[id]?.firstIndex(where: { $0.externalId == externalId }) else { return false }
        items[id]!.remove(at: i)
        return true
    }
    func onChange(_ h: @escaping () -> Void) { handler = h }
}

func exec(_ args: [String], _ store: ReminderStore) async -> (Int32, [String]) {
    var lines: [String] = []
    let code = await run(args, store: store) { lines.append($0) }
    return (code, lines)
}

func parse(_ s: String) -> Any { try! JSONSerialization.jsonObject(with: Data(s.utf8), options: .fragmentsAllowed) }

struct CommandsTests {
    @Test func fetch_maps_date_only_due() async {
        let s = FakeStore()
        s.items["L1"] = [RawReminder(externalId: "a", title: "x", due: DateComponents(year: 2026, month: 3, day: 9))]
        let (code, out) = await exec(["fetch", "--list", "L1"], s)
        #expect(code == 0)
        #expect(out == [#"[{"due":{"date":"2026-03-09"},"external_id":"a","notes":null,"title":"x"}]"#])
    }

    @Test func fetch_maps_datetime_due_with_tz() async {
        let s = FakeStore()
        let tz = TimeZone(identifier: "America/New_York")!
        s.items["L1"] = [RawReminder(externalId: "a", title: "x", due: DateComponents(timeZone: tz, year: 2026, month: 1, day: 5, hour: 9, minute: 30))]
        let (_, out) = await exec(["fetch", "--list", "L1"], s)
        let due = (parse(out[0]) as! [[String: Any]])[0]["due"] as! [String: String]
        #expect(due == ["datetime": "2026-01-05T14:30:00Z", "tz": "America/New_York"])
    }

    @Test func fetch_excludes_completed() async {
        let s = FakeStore()
        s.items["L1"] = [RawReminder(externalId: "a", title: "open"), RawReminder(externalId: "b", title: "done", isCompleted: true)]
        let (_, out) = await exec(["fetch", "--list", "L1"], s)
        #expect((parse(out[0]) as! [[String: Any]]).map { $0["external_id"] as! String } == ["a"])
    }

    @Test func fetch_notes_with_newlines_and_quotes_round_trip() async {
        let s = FakeStore()
        let notes = "line1\n\"quoted\" \\ back\ttab"
        s.items["L1"] = [RawReminder(externalId: "a", title: "x", notes: notes)]
        let (_, out) = await exec(["fetch", "--list", "L1"], s)
        #expect((parse(out[0]) as! [[String: Any]])[0]["notes"] as? String == notes)
    }

    @Test func fetch_unknown_list_is_json_error() async {
        let (code, out) = await exec(["fetch", "--list", "nope"], FakeStore())
        #expect(code == 1)
        #expect(out == [#"{"error":"unknown list: nope"}"#])
    }

    @Test func delete_reports_missing() async {
        let s = FakeStore()
        s.items["L1"] = [RawReminder(externalId: "a", title: "x")]
        let (code, out) = await exec(["delete", "--list", "L1", "a", "zzz"], s)
        #expect(code == 0)
        #expect(out == [#"{"deleted":["a"],"missing":["zzz"]}"#])
    }

    @Test func delete_ignores_same_id_in_other_list() async {
        let s = FakeStore()
        s.items["L2"] = [RawReminder(externalId: "a", title: "other list")]
        let (code, out) = await exec(["delete", "--list", "L1", "a"], s)
        #expect(code == 0)
        #expect(out == [#"{"deleted":[],"missing":["a"]}"#])
        #expect(s.items["L2"]!.count == 1)
    }

    @Test func errors_are_json() async {
        let (code, out) = await exec(["fetch"], FakeStore())
        #expect(code == 1)
        #expect(parse(out[0]) as? [String: String] == ["error": "fetch requires --list <id>"])
    }

    @Test func unknown_subcommand_is_json_error() async {
        let (code, out) = await exec(["frobnicate"], FakeStore())
        #expect(code == 1)
        #expect(out == [#"{"error":"unknown subcommand: frobnicate"}"#])
    }

    @Test func lists_and_watch() async {
        let s = FakeStore()
        let (_, out) = await exec(["lists"], s)
        #expect(out == [#"[{"id":"L1","title":"T-L1"}]"#])
        var lines: [String] = []
        _ = await run(["watch", "--list", "L1"], store: s) { lines.append($0) }
        s.handler?()
        #expect(lines == [#"{"changed":true}"#])
    }
}
