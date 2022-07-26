#include "app_log.h"

#include <cstddef>
#include <string_view>

AppLog::AppLog() : auto_scroll(true) { clear(); }

void AppLog::clear() {
    std::scoped_lock lock(mutex);

    buffer.clear();
    line_offsets.clear();
}

void AppLog::add(const char* message) {
    std::scoped_lock lock(mutex);
    std::string_view message_view(message);

    std::size_t line_start = 0;
    for (std::size_t i = 0; i < message_view.size(); i++) {
        if (message_view[i] == '\n') {
            line_offsets.push_back(buffer.size());
            buffer.append(&message_view[line_start], &message_view[i]);
            line_start = i + 1;
        }
    }
}

void AppLog::draw(const char* title, bool& open) {
    if (!open) {
        return;
    }

    if (!ImGui::Begin(title, &open, ImGuiWindowFlags_NoFocusOnAppearing)) {
        ImGui::End();
        return;
    }

    if (ImGui::BeginPopup("Options")) {
        ImGui::Checkbox("Auto-scroll", &auto_scroll);

        ImGui::EndPopup();
    }

    if (ImGui::Button("Options")) {
        ImGui::OpenPopup("Options");
    }

    ImGui::SameLine();
    bool do_clear = ImGui::Button("Clear");
    ImGui::SameLine();
    bool do_copy = ImGui::Button("Copy");

    ImGui::Separator();
    ImGui::BeginChild("scrolling", ImVec2(0, 0), false, ImGuiWindowFlags_HorizontalScrollbar);

    if (do_clear) clear();

    ImGui::PushStyleVar(ImGuiStyleVar_ItemSpacing, ImVec2(0.0f, 0.0f));

    if (do_copy) ImGui::LogToClipboard();

    {
        std::scoped_lock lock(mutex);

        const char* buf_start = buffer.begin();
        const char* buf_end = buffer.end();
        ImGuiListClipper clipper;
        clipper.Begin(line_offsets.Size);
        while (clipper.Step()) {
            for (int line = clipper.DisplayStart; line < clipper.DisplayEnd; line++) {
                const char* line_start = buf_start + line_offsets[line];
                const char* line_end = (line + 1 < line_offsets.Size) ? (buf_start + line_offsets[line + 1]) : buf_end;
                ImGui::TextUnformatted(line_start, line_end);
            }
        }
        clipper.End();
    }

    if (do_copy) ImGui::LogFinish();

    ImGui::PopStyleVar();

    if (auto_scroll && ImGui::GetScrollY() >= ImGui::GetScrollMaxY()) {
        ImGui::SetScrollHereY(1.0f);
    }

    ImGui::EndChild();
    ImGui::End();
}
