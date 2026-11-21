import EventKit
import Foundation

public final class EventKitStore: ReminderStore {
    private let store = EKEventStore()
    public init() {}

    public func requestAccess() async throws -> Bool {
        try await store.requestFullAccessToReminders()
    }

    public func lists() throws -> [ReminderList] {
        store.calendars(for: .reminder).map { ReminderList(id: $0.calendarIdentifier, title: $0.title) }
    }

    public func reminders(inList id: String) async throws -> [RawReminder] {
        guard let cal = store.calendars(for: .reminder).first(where: { $0.calendarIdentifier == id }) else {
            throw HelperError("unknown list: \(id)")
        }
        let predicate = store.predicateForReminders(in: [cal])
        return await withCheckedContinuation { cont in
            store.fetchReminders(matching: predicate) { found in
                cont.resume(returning: (found ?? []).map {
                    RawReminder(externalId: $0.calendarItemExternalIdentifier, title: $0.title ?? "",
                                notes: $0.notes, isCompleted: $0.isCompleted, due: $0.dueDateComponents)
                })
            }
        }
    }

    public func delete(externalId: String, inList id: String) throws -> Bool {
        let found = store.calendarItems(withExternalIdentifier: externalId)
            .compactMap { $0 as? EKReminder }
            .filter { $0.calendar.calendarIdentifier == id }
        if found.isEmpty { return false }
        for r in found { try store.remove(r, commit: false) }
        try store.commit()
        return true
    }

    public func onChange(_ handler: @escaping () -> Void) {
        NotificationCenter.default.addObserver(forName: .EKEventStoreChanged, object: store, queue: nil) { _ in handler() }
    }
}
