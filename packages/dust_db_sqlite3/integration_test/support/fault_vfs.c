/* Test-only VFS: delegate real I/O and fail one selected native boundary. */
#include <sqlite3.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
  sqlite3_file file;
  sqlite3_file *native;
  int target;
} FaultFile;

static sqlite3_vfs wrapped;
static sqlite3_vfs *original;
static sqlite3_io_methods methods;
static struct {
  int operation, target, persistent, partial, hits, partial_writes, partial_bytes;
} fault;
static unsigned generation;

static void read_configuration(void) {
  const char *path = getenv("DUST_TEST_VFS_CONTROL");
  if (path == NULL) return;
  FILE *input = fopen(path, "r");
  if (input == NULL) return;
  unsigned next;
  int operation, target, persistent, partial;
  int fields = fscanf(input, "%u%d%d%d%d", &next, &operation, &target,
                      &persistent, &partial);
  fclose(input);
  if (fields == 5 && next != generation && operation >= 0 && operation <= 4 &&
      target >= 0 && target <= 3 && (persistent == 0 || persistent == 1) &&
      (partial == 0 || partial == 1)) {
    generation = next;
    fault.operation = operation;
    fault.target = target;
    fault.persistent = persistent;
    fault.partial = partial;
    if (operation != 0) {
      fault.hits = fault.partial_writes = fault.partial_bytes = 0;
    }
  }
}

static void record_hit(void) {
  const char *path = getenv("DUST_TEST_VFS_CONTROL");
  char marker[4096];
  if (path == NULL || snprintf(marker, sizeof(marker), "%s.hits", path) >=
                          (int)sizeof(marker)) return;
  FILE *output = fopen(marker, "w");
  if (output != NULL) {
    fprintf(output, "%d %d %d", fault.hits, fault.partial_writes, fault.partial_bytes);
    fclose(output);
  }
}

static FaultFile *file(sqlite3_file *p) { return (FaultFile *)p; }
static int fail_io(sqlite3_file *p, int operation) {
  read_configuration();
  if (fault.operation != operation || fault.target != file(p)->target)
    return 0;
  ++fault.hits;
  record_hit();
  if (!fault.persistent) fault.operation = 0;
  return 1;
}

static int close_file(sqlite3_file *p) {
  int rc = file(p)->native->pMethods->xClose(file(p)->native);
  p->pMethods = NULL;
  return rc;
}
static int read_file(sqlite3_file *p, void *b, int n, sqlite3_int64 offset) {
  if (fail_io(p, 1)) return SQLITE_IOERR_READ;
  return file(p)->native->pMethods->xRead(file(p)->native, b, n, offset);
}
static int write_file(sqlite3_file *p, const void *b, int n, sqlite3_int64 offset) {
  if (fail_io(p, 2)) {
    if (fault.partial && n > 1) {
      int rc = file(p)->native->pMethods->xWrite(file(p)->native, b, n / 2, offset);
      if (rc != SQLITE_OK) return rc;
      ++fault.partial_writes;
      fault.partial_bytes += n / 2;
      record_hit();
    }
    return SQLITE_IOERR_WRITE;
  }
  return file(p)->native->pMethods->xWrite(file(p)->native, b, n, offset);
}
static int truncate_file(sqlite3_file *p, sqlite3_int64 n) {
  if (fail_io(p, 3)) return SQLITE_IOERR_TRUNCATE;
  return file(p)->native->pMethods->xTruncate(file(p)->native, n);
}
static int sync_file(sqlite3_file *p, int flags) {
  if (fail_io(p, 4)) return SQLITE_IOERR_FSYNC;
  return file(p)->native->pMethods->xSync(file(p)->native, flags);
}
static int size_file(sqlite3_file *p, sqlite3_int64 *n) {
  return file(p)->native->pMethods->xFileSize(file(p)->native, n);
}
static int lock_file(sqlite3_file *p, int mode) {
  return file(p)->native->pMethods->xLock(file(p)->native, mode);
}
static int unlock_file(sqlite3_file *p, int mode) {
  return file(p)->native->pMethods->xUnlock(file(p)->native, mode);
}
static int reserved_file(sqlite3_file *p, int *result) {
  return file(p)->native->pMethods->xCheckReservedLock(file(p)->native, result);
}
static int control_file(sqlite3_file *p, int operation, void *arg) {
  return file(p)->native->pMethods->xFileControl(file(p)->native, operation, arg);
}
static int sector_file(sqlite3_file *p) {
  return file(p)->native->pMethods->xSectorSize(file(p)->native);
}
static int device_file(sqlite3_file *p) {
  (void)p;
  /* Injected partial writes cannot promise native atomic-write capabilities. */
  return 0;
}
static int shm_map(sqlite3_file *p, int page, int size, int extend, void volatile **out) {
  return file(p)->native->pMethods->xShmMap(file(p)->native, page, size, extend, out);
}
static int shm_lock(sqlite3_file *p, int offset, int n, int flags) {
  return file(p)->native->pMethods->xShmLock(file(p)->native, offset, n, flags);
}
static void shm_barrier(sqlite3_file *p) {
  file(p)->native->pMethods->xShmBarrier(file(p)->native);
}
static int shm_unmap(sqlite3_file *p, int remove) {
  return file(p)->native->pMethods->xShmUnmap(file(p)->native, remove);
}
static int fetch_file(sqlite3_file *p, sqlite3_int64 offset, int n, void **out) {
  if (file(p)->native->pMethods->xFetch == NULL) {
    *out = NULL;
    return SQLITE_OK;
  }
  return file(p)->native->pMethods->xFetch(file(p)->native, offset, n, out);
}
static int unfetch_file(sqlite3_file *p, sqlite3_int64 offset, void *out) {
  if (file(p)->native->pMethods->xUnfetch == NULL) return SQLITE_OK;
  return file(p)->native->pMethods->xUnfetch(file(p)->native, offset, out);
}

static int open_file(sqlite3_vfs *vfs, const char *name, sqlite3_file *p,
                     int flags, int *out_flags) {
  (void)vfs;
  FaultFile *f = file(p);
  f->native = (sqlite3_file *)((char *)p + sizeof(FaultFile));
  int rc = original->xOpen(original, name, f->native, flags, out_flags);
  if (rc != SQLITE_OK) return rc;
  if (f->native->pMethods->iVersion < 3 ||
      f->native->pMethods->xShmMap == NULL ||
      f->native->pMethods->xShmLock == NULL ||
      f->native->pMethods->xShmBarrier == NULL ||
      f->native->pMethods->xShmUnmap == NULL) {
    f->native->pMethods->xClose(f->native);
    return SQLITE_CANTOPEN;
  }
  f->target = flags & SQLITE_OPEN_MAIN_DB ? 1 :
      flags & SQLITE_OPEN_MAIN_JOURNAL ? 2 : flags & SQLITE_OPEN_WAL ? 3 : 0;
  p->pMethods = &methods;
  return SQLITE_OK;
}

/* No extension loading or Dart bindings: confined to an isolated test child. */
__attribute__((constructor)) static void install(void) {
  original = sqlite3_vfs_find(NULL);
  if (original == NULL) return;
  wrapped = *original;
  wrapped.zName = "dust_fault";
  wrapped.pNext = NULL;
  wrapped.szOsFile = (int)sizeof(FaultFile) + original->szOsFile;
  wrapped.xOpen = open_file;
  methods = (sqlite3_io_methods){3, close_file, read_file, write_file,
    truncate_file, sync_file, size_file, lock_file, unlock_file,
    reserved_file, control_file, sector_file, device_file,
    shm_map, shm_lock, shm_barrier, shm_unmap, fetch_file, unfetch_file};
  sqlite3_vfs_register(&wrapped, 0);
}
