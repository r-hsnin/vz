use super::detect::detect_columns;
use super::detect::looks_like_space_format;
use super::is_separator_line;
use super::parse::load_space;

// --- Detection Tests ---

#[test]
fn test_looks_like_space_format_kubectl_top() {
    let content = "NAME        CPU(cores)   MEMORY(bytes)\npod1        100m         256Mi\n";
    assert!(looks_like_space_format(content));
}

#[test]
fn test_looks_like_space_format_not_csv() {
    let content = "name,cpu,mem\npod1,100m,256Mi\n";
    assert!(!looks_like_space_format(content));
}

#[test]
fn test_looks_like_space_format_not_tsv() {
    let content = "name\tcpu\tmem\npod1\t100m\t256Mi\n";
    assert!(!looks_like_space_format(content));
}

#[test]
fn test_looks_like_space_format_single_line() {
    let content = "NAME        CPU(cores)   MEMORY(bytes)\n";
    assert!(!looks_like_space_format(content));
}

#[test]
fn test_looks_like_space_format_with_separators() {
    let content = "Name        Score\n----------  -----\nAlice       95\n";
    assert!(looks_like_space_format(content));
}

// --- Column Detection Tests ---

#[test]
fn test_detect_columns_simple() {
    let cols = detect_columns("NAME        CPU(cores)   MEMORY(bytes)");
    assert_eq!(cols.len(), 3);
    assert_eq!(cols[0].name, "NAME");
    assert_eq!(cols[1].name, "CPU(cores)");
    assert_eq!(cols[2].name, "MEMORY(bytes)");
}

#[test]
fn test_detect_columns_multi_word_header() {
    // "Mounted on" has only 1 space — should be one column (wide-gap strategy)
    // Header: 7 tokens, wide-gap gives 6 columns (6+2=8 >= 7) → uses wide-gap
    let cols = detect_columns("Filesystem      Size  Used  Avail  Use%  Mounted on");
    assert_eq!(cols.len(), 6);
    assert_eq!(cols[0].name, "Filesystem");
    assert_eq!(cols[5].name, "Mounted on");
}

#[test]
fn test_detect_columns_varying_gaps() {
    // lsblk: 7 tokens, wide-gap gives 3 columns (3+2=5 < 7) → falls back to single-space
    let cols = detect_columns("NAME   MAJ:MIN RM   SIZE RO TYPE MOUNTPOINTS");
    assert_eq!(cols.len(), 7);
    assert_eq!(cols[0].name, "NAME");
    assert_eq!(cols[1].name, "MAJ:MIN");
    assert_eq!(cols[6].name, "MOUNTPOINTS");
}

// --- Parsing Tests ---

#[test]
fn test_load_space_kubectl_top() {
    let content = "\
NAME                                    CPU(cores)   MEMORY(bytes)
frontend-deploy-7b4c9f8d6-abc12        100m         256Mi
backend-deploy-5d6e7f8a9-def34         200m         512Mi
redis-master-0                          50m          128Mi
";
    let data = load_space(content, false).unwrap();
    assert_eq!(data.headers, vec!["NAME", "CPU(cores)", "MEMORY(bytes)"]);
    assert_eq!(data.rows.len(), 3);
    assert_eq!(data.rows[0][0], "frontend-deploy-7b4c9f8d6-abc12");
    assert_eq!(data.rows[0][1], "100m");
    assert_eq!(data.rows[0][2], "256Mi");
    assert_eq!(data.rows[2][0], "redis-master-0");
    assert_eq!(data.rows[2][1], "50m");
}

#[test]
fn test_load_space_separator_lines_skipped() {
    let content = "\
Name        Score   Grade
----------  ------  -----
Alice       95      A
Bob         82      B
Charlie     71      C
";
    let data = load_space(content, false).unwrap();
    assert_eq!(data.headers, vec!["Name", "Score", "Grade"]);
    assert_eq!(data.rows.len(), 3);
    assert_eq!(data.rows[0], vec!["Alice", "95", "A"]);
}

#[test]
fn test_load_space_empty_trailing_values() {
    let content = "\
NAME        STATUS    ERROR
service-a   Running
service-b   Failed    timeout
service-c   Running
";
    let data = load_space(content, false).unwrap();
    assert_eq!(data.headers, vec!["NAME", "STATUS", "ERROR"]);
    assert_eq!(data.rows.len(), 3);
    assert_eq!(data.rows[0][0], "service-a");
    assert_eq!(data.rows[0][1], "Running");
    assert_eq!(data.rows[0][2], "");
    assert_eq!(data.rows[1][2], "timeout");
}

#[test]
fn test_load_space_single_row() {
    let content = "\
NAME         CPU    MEM
my-pod       50m    128Mi
";
    let data = load_space(content, false).unwrap();
    assert_eq!(data.headers, vec!["NAME", "CPU", "MEM"]);
    assert_eq!(data.rows.len(), 1);
    assert_eq!(data.rows[0], vec!["my-pod", "50m", "128Mi"]);
}

#[test]
fn test_load_space_no_header() {
    let content = "\
100   200   300
150   250   350
200   300   400
";
    let data = load_space(content, true).unwrap();
    assert_eq!(data.headers, vec!["col1", "col2", "col3"]);
    assert_eq!(data.rows.len(), 3);
    assert_eq!(data.rows[0], vec!["100", "200", "300"]);
}

#[test]
fn test_load_space_auto_detect_headerless() {
    // All-numeric first row: auto-treat as headerless
    let content = "\
100   200   300
150   250   350
";
    let data = load_space(content, false).unwrap();
    assert_eq!(data.headers, vec!["col1", "col2", "col3"]);
    assert_eq!(data.rows.len(), 2);
}

#[test]
fn test_load_space_lsblk() {
    let content = "\
NAME   MAJ:MIN RM   SIZE RO TYPE MOUNTPOINTS
sda      8:0    0 476.9G  0 disk
sda1     8:1    0   512M  0 part /boot/efi
sda2     8:2    0 476.4G  0 part /
";
    let data = load_space(content, false).unwrap();
    assert_eq!(data.headers.len(), 7);
    assert_eq!(data.headers[0], "NAME");
    assert_eq!(data.headers[1], "MAJ:MIN");
    assert_eq!(data.headers[2], "RM");
    assert_eq!(data.headers[5], "TYPE");
    assert_eq!(data.headers[6], "MOUNTPOINTS");
    assert_eq!(data.rows.len(), 3);
    // With single-space fallback, each token is a column
    assert_eq!(data.rows[0][0], "sda");
    assert_eq!(data.rows[0][5], "disk");
    assert_eq!(data.rows[0][6], ""); // no mountpoint for disk
    assert_eq!(data.rows[2][6], "/");
}

#[test]
fn test_load_space_df_h() {
    let content = "\
Filesystem      Size  Used  Avail  Use%  Mounted on
/dev/sda1        50G   35G    15G   70%  /
tmpfs           7.8G     0   7.8G    0%  /dev/shm
";
    let data = load_space(content, false).unwrap();
    assert_eq!(data.headers.len(), 6);
    assert_eq!(data.headers[0], "Filesystem");
    assert_eq!(data.headers[5], "Mounted on");
    assert_eq!(data.rows.len(), 2);
    assert_eq!(data.rows[0][0], "/dev/sda1");
    assert_eq!(data.rows[0][4], "70%");
    assert_eq!(data.rows[0][5], "/");
    assert_eq!(data.rows[1][5], "/dev/shm");
}

#[test]
fn test_load_space_ps_aux_last_column_has_spaces() {
    // With single-space tokenization fallback, each whitespace-separated
    // token becomes a column. For ps aux, the COMMAND column with spaces
    // gets split into multiple columns.
    // The primary use case (kubectl top) handles this correctly via wide-gap.
    // For ps aux, the user can use -f space with custom column selection.
    let content = "\
USER       PID  %CPU  %MEM  COMMAND
root         1   0.0   0.1  /sbin/init
www-data  1234   1.5   2.3  nginx
";
    let data = load_space(content, false).unwrap();
    assert_eq!(data.headers[0], "USER");
    assert_eq!(data.headers[4], "COMMAND");
    assert_eq!(data.rows.len(), 2);
    assert_eq!(data.rows[0][0], "root");
    assert_eq!(data.rows[0][4], "/sbin/init");
    assert_eq!(data.rows[1][4], "nginx");
}

#[test]
fn test_load_space_empty_content() {
    let data = load_space("", false).unwrap();
    assert_eq!(data.headers.len(), 0);
    assert_eq!(data.rows.len(), 0);
}

#[test]
fn test_load_space_trims_whitespace() {
    let content = "\
NAME        VALUE
foo         bar
baz         qux
";
    let data = load_space(content, false).unwrap();
    assert_eq!(data.rows[0][0], "foo");
    assert_eq!(data.rows[0][1], "bar");
}

// --- Separator Line Detection ---

#[test]
fn test_is_separator_line_dashes() {
    assert!(is_separator_line("----------  ------  -----"));
}

#[test]
fn test_is_separator_line_equals() {
    assert!(is_separator_line("==========  ======  ====="));
}

#[test]
fn test_is_separator_line_mixed() {
    assert!(is_separator_line("---+---+---"));
}

#[test]
fn test_is_separator_line_not_data() {
    assert!(!is_separator_line("Alice       95      A"));
}

#[test]
fn test_is_separator_line_empty() {
    assert!(!is_separator_line(""));
}
