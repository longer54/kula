pragma Singleton
import QtQuick

// UI translations. The key is the English text (default); the `dict` map holds the Polish equivalents.
QtObject {
    id: lg
    property string mode: "auto"
    readonly property bool pl: mode === "pl" || (mode === "auto" && Qt.locale().name.indexOf("pl") === 0)

    readonly property var dict: ({
        "   errors: ": "   błędy: ",
        "   matched to photos: ": "   dopasowane do zdjęć: ",
        "   processed: ": "   przetworzone: ",
        "   to read: ": "   do odczytu: ",
        " (missing)": " (brak)",
        " (present)": " (jest)",
        "#  or  Delete  to trash      Ctrl+wheel  grid size      F11  full screen      Esc  back / deselect": "#  lub  Delete  do kosza      Ctrl+kółko  rozmiar siatki      F11  pełny ekran      Esc  wstecz / odznacz",
        ", name ": ", nazwa ",
        ", none ": ", bez daty ",
        "/   search\n←  →   previous / next (viewer)\nz   + −   0   zoom\ni   Info panel\nSpace   video: play / pause\nx   select under cursor     Shift+click   range     Ctrl+A   all\nf   favorite     Shift+A   archive     #  or  Delete   to trash\nCtrl+wheel   grid size     F11   full screen     Esc   back": "/   szukaj\n←  →   poprzednie / następne (przeglądarka)\nz   + −   0   powiększenie\ni   panel Informacje\nSpacja   wideo: odtwarzaj / pauza\nx   zaznacz pod kursorem     Shift+klik   zakres     Ctrl+A   wszystko\nf   ulubione     Shift+A   archiwum     #  lub  Delete   do kosza\nCtrl+kółko   rozmiar siatki     F11   pełny ekran     Esc   wstecz",
        "/  search      ← →  previous / next      z  + −  zoom      i  Info      Space  video\n": "/  szukaj      ← →  poprzednie / następne      z  + −  powiększenie      i  Informacje      Spacja  wideo\n",
        "AI data: ": "Dane AI: ",
        "Add a description": "Dodaj opis",
        "Add to album": "Dodaj do albumu",
        "Add to favorites (F)": "Dodaj do ulubionych (F)",
        "Added to album “%1”: %2": "Dodano do albumu „%1”: %2",
        "Album title": "Tytuł albumu",
        "Albums": "Albumy",
        "Approximate date: day received in WhatsApp (from file name)": "Data przybliżona: dzień otrzymania w WhatsApp (z nazwy pliku)",
        "Approximate date: file save time (timestamp from name)": "Data przybliżona: czas zapisu pliku (znacznik z nazwy)",
        "Archive": "Archiwum",
        "Archive|action": "Archiwizuj",
        "Archived: ": "Archiwum: ",
        "Automatic": "Automatyczny",
        "Cancel": "Anuluj",
        "Content search: ": "Wyszukiwanie po treści: ",
        "Content: ": "Treść: ",
        "Copy": "Kopiuj",
        "Copy path": "Kopiuj ścieżkę",
        "Create": "Utwórz",
        "Dark": "Ciemny",
        "Date source (last scan): EXIF ": "Źródło daty (ostatni skan): EXIF ",
        "Delete permanently": "Usuń na zawsze",
        "Delete permanently…": "Usuń na zawsze…",
        "Deleted permanently": "Usunięto na zawsze",
        "Delete…": "Usuń…",
        "Describe what you are looking for (e.g. beach at sunset)": "Opisz, co szukasz (np. plaża o zachodzie)",
        "Done: ": "Gotowe: ",
        "Enter — save": "Enter — zapisz",
        "Favorites": "Ulubione",
        "Favorites changed: ": "Zmieniono ulubione: ",
        "Files found: ": "Znalezione pliki: ",
        "Hide rare people": "Ukryj rzadkie osoby",
        "Hide side panel (Ctrl+B)": "Schowaj panel boczny (Ctrl+B)",
        "Index: ": "Indeks: ",
        "Info": "Informacje",
        "JSON files: ": "Pliki JSON: ",
        "Keyboard shortcuts": "Skróty klawiszowe",
        "Language": "Język",
        "Light": "Jasny",
        "Loading…": "Ładowanie…",
        "Merge": "Połącz",
        "Merged %1 people (%2 faces)": "Połączono %1 osoby (%2 twarzy)",
        "Move %1 to trash?": "Przenieść do kosza %1?",
        "Move to trash": "Przenieś do kosza",
        "Move to trash (Delete)": "Przenieś do kosza (Delete)",
        "Moved to trash": "Przeniesiono do kosza",
        "New album": "Nowy album",
        "New album…": "Nowy album…",
        "No albums yet. Create the first one with the button above.": "Brak albumów. Utwórz pierwszy przyciskiem powyżej.",
        "No date": "Bez daty",
        "No date in the file, JSON or name": "Brak daty w pliku, JSON-ie i nazwie",
        "No photos in this view.": "Brak zdjęć w tym widoku.",
        "No recognized people (ai.sqlite).": "Brak rozpoznanych osób (ai.sqlite).",
        "No results for this query.": "Brak wyników dla tego zapytania.",
        "No results.": "Brak wyników.",
        "Opening folder…": "Otwieram folder…",
        "Path copied": "Skopiowano ścieżkę",
        "People": "Osoby",
        "Permanently delete %1?": "Usunąć NA ZAWSZE %1?",
        "Person": "Osoba",
        "Photo folder": "Folder ze zdjęciami",
        "Photo folder (read-only — Kula never changes anything in it)": "Katalog ze zdjęciami (tylko odczyt — Kula nigdy nic w nim nie zmienia)",
        "Photo folder…": "Folder ze zdjęciami…",
        "Photo info (I)": "Informacje o zdjęciu (I)",
        "Photos": "Zdjęcia",
        "Recently added": "Ostatnio dodane",
        "Remove from album": "Usuń z albumu",
        "Remove from favorites (F)": "Usuń z ulubionych (F)",
        "Removed from favorites": "Usunięto z ulubionych",
        "Rename": "Zmień nazwę",
        "Restore": "Przywróć",
        "Restore from archive": "Przywróć z archiwum",
        "Restored": "Przywrócono",
        "Results: ": "Wyniki: ",
        "Save and scan": "Zapisz i skanuj",
        "Scan now": "Skanuj teraz",
        "Scan started": "Skanowanie rozpoczęte",
        "Scan: ": "Skan: ",
        "Scanning library…": "Skanowanie biblioteki…",
        "Scanning: ": "Skanowanie: ",
        "Search by content (AI)": "Szukaj po treści (AI)",
        "Search photos": "Szukaj zdjęć",
        "Searching by content…": "Szukam po treści…",
        "Select a single photo": "Wskaż jedno zdjęcie",
        "Select photos first": "Najpierw zaznacz zdjęcia",
        "Select several people with the circle and click “Merge”. Click a name to rename it.": "Zaznacz kółkiem kilka osób i kliknij „Połącz”. Kliknij imię, aby je zmienić.",
        "Selected: ": "Wybrano ",
        "Settings": "Ustawienia",
        "Show all (hidden: %1)": "Pokaż wszystkie (ukryte: %1)",
        "Show in folder": "Pokaż w folderze",
        "Show side panel (Ctrl+B)": "Pokaż panel boczny (Ctrl+B)",
        "Sort: ": "Sortuj: ",
        "System": "Systemowy",
        "Takeout album — name is read-only": "Album z Takeout — nazwa tylko do odczytu",
        "Takeout metadata folder (JSON files; empty = none)": "Katalog metadanych Takeout (pliki JSON; puste = brak)",
        "The files will go to the system trash (you can restore them from the “Trash” view).": "Pliki trafią do kosza systemowego (można je stąd przywrócić w ekranie „Kosz”).",
        "The operation failed": "Operacja się nie udała",
        "Theme": "Motyw",
        "This cannot be undone. The files will disappear from the system trash and from disk.": "Tej operacji nie można cofnąć. Pliki znikną z kosza systemowego i z dysku.",
        "This photo cannot be displayed (unsupported format or damaged file)": "Nie można wyświetlić tego zdjęcia (format lub uszkodzony plik)",
        "Thumbnails: ": "Miniatury: ",
        "Trash": "Kosz",
        "Unnamed": "Bez nazwy",
        "Version: ": "Wersja: ",
        "Videos": "Filmy",
        "\n\nFiles affected by the error were left untouched (nothing was deleted permanently).": "\n\nPliki, których dotyczy błąd, zostały nietknięte (nic nie skasowano trwale).",
        "available": "dostępne",
        "browsing folders": "przeglądanie katalogów",
        "cleaning up": "porządkowanie",
        "idle": "bezczynny",
        "items": "elem.",
        "loading JSON": "wczytywanie JSON",
        "name": "nazwa",
        "newest": "najnowsze",
        "reading metadata": "odczyt metadanych",
        "unavailable": "niedostępne",
        "x  select (under cursor)      Shift+click  range      Ctrl+A  all      f  favorite      Shift+A  archive\n": "x  zaznacz (pod kursorem)      Shift+klik  zakres      Ctrl+A  wszystko      f  ulubione      Shift+A  archiwum\n",
        "←  Back": "←  Wróć",
        "★ Favorite": "★ Ulubione",
        "＋  Create album": "＋  Utwórz album",
        "＋ Album": "＋ Album"
    })

    // nouns: [en 1, en N, pl 1, pl 2-4, pl 5+]
    readonly property var nouns: ({
        item:  ["item", "items", "element", "elementy", "elementów"],
        file:  ["file", "files", "plik", "pliki", "plików"],
        photo: ["photo", "photos", "zdjęcie", "zdjęcia", "zdjęć"]
    })

    // a key may carry a context after "|" (e.g. "Archive|action"); it is stripped from the result
    function t(s) { var r = (pl && dict[s] !== undefined) ? dict[s] : s; var k = r.indexOf("|"); return k < 0 ? r : r.substring(0, k) }

    function f(s) {
        var r = t(s)
        for (var i = 1; i < arguments.length; i++) r = r.split("%" + i).join(arguments[i])
        return r
    }

    function noun(n, key) {
        var w = nouns[key]
        if (!pl) return n === 1 ? w[0] : w[1]
        if (n === 1) return w[2]
        var d = n % 10, h = n % 100
        return (d >= 2 && d <= 4 && (h < 10 || h >= 20)) ? w[3] : w[4]
    }

    function cnt(n, key) { return Theme.fmt(n) + " " + noun(n, key) }
}
