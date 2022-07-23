#include "app_log.h"

AppLog::AppLog() : auto_scroll(true) {
    clear();

    for (int i = 0; i < 36; i++) {
        add("info: Hello world!\n");
    }
}

void AppLog::clear() {
    buffer.clear();
    line_offsets.clear();
    line_offsets.push_back(0);
}

void AppLog::add(const char* message) {
    int prev_size = buffer.size();
    buffer.append(message);
    for (int i = prev_size; i < buffer.size(); i++) {
        if (buffer[i] == '\n') {
            line_offsets.push_back(i + 1);
        }
    }
}

void AppLog::draw(const char* title, bool& open) {
    if (!open) {
        return;
    }

    ImGui::SetNextWindowSize(ImVec2(400.0f, 250.0f), ImGuiCond_FirstUseEver);
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

    const char* buf_start = buffer.begin();
    const char* buf_end = buffer.end();
    ImGuiListClipper clipper;
    clipper.Begin(line_offsets.Size);
    while (clipper.Step()) {
        for (int line = clipper.DisplayStart; line < clipper.DisplayEnd; line++) {
            const char* line_start = buf_start + line_offsets[line];
            const char* line_end = (line + 1 < line_offsets.Size) ? (buf_start + line_offsets[line + 1] - 1) : buf_end;
            ImGui::TextUnformatted(line_start, line_end);
        }
    }
    clipper.End();

    if (do_copy) ImGui::LogFinish();

    ImGui::PopStyleVar();

    if (auto_scroll && ImGui::GetScrollY() >= ImGui::GetScrollMaxY()) {
        ImGui::SetScrollHereY(1.0f);
    }

    ImGui::EndChild();
    ImGui::End();
}
