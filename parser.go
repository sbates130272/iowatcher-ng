package main

/*

 typedef __signed__ char __s8;
 typedef unsigned char __u8;

 typedef __signed__ short __s16;
 typedef unsigned short __u16;

 typedef __signed__ int __s32;
 typedef unsigned int __u32;

 #ifdef __GNUC__
 __extension__ typedef __signed__ long long __s64;
 __extension__ typedef unsigned long long __u64;
 #else
 typedef __signed__ long long __s64;
 typedef unsigned long long __u64;
 #endif

 // Trace categories
 enum {
	BLK_TC_READ	= 1 << 0,	// reads
	BLK_TC_WRITE	= 1 << 1,	// writes
	BLK_TC_FLUSH	= 1 << 2,	// flush
	BLK_TC_SYNC	= 1 << 3,	// sync
	BLK_TC_QUEUE	= 1 << 4,	// queueing/merging
	BLK_TC_REQUEUE	= 1 << 5,	// requeueing
	BLK_TC_ISSUE	= 1 << 6,	// issue
	BLK_TC_COMPLETE	= 1 << 7,	// completions
	BLK_TC_FS	= 1 << 8,	// fs requests
	BLK_TC_PC	= 1 << 9,	// pc requests
	BLK_TC_NOTIFY	= 1 << 10,	// special message
	BLK_TC_AHEAD	= 1 << 11,	// readahead
	BLK_TC_META	= 1 << 12,	// metadata
	BLK_TC_DISCARD	= 1 << 13,	// discard requests
	BLK_TC_DRV_DATA	= 1 << 14,	// binary driver data
	BLK_TC_FUA	= 1 << 15,	// fua requests

	BLK_TC_END	= 1 << 15,	// we've run out of bits!
};

#define BLK_TC_SHIFT		(16)
#define BLK_TC_ACT(act)		((act) << BLK_TC_SHIFT)

// Basic trace actions
enum {
	__BLK_TA_QUEUE = 1,		// queued
	__BLK_TA_BACKMERGE,		// back merged to existing rq
	__BLK_TA_FRONTMERGE,		// front merge to existing rq
	__BLK_TA_GETRQ,			// allocated new request
	__BLK_TA_SLEEPRQ,		// sleeping on rq allocation
	__BLK_TA_REQUEUE,		// request requeued
	__BLK_TA_ISSUE,			// sent to driver
	__BLK_TA_COMPLETE,		// completed by driver
	__BLK_TA_PLUG,			// queue was plugged
	__BLK_TA_UNPLUG_IO,		// queue was unplugged by io
	__BLK_TA_UNPLUG_TIMER,		// queue was unplugged by timer
	__BLK_TA_INSERT,		// insert request
	__BLK_TA_SPLIT,			// bio was split
	__BLK_TA_BOUNCE,		// bio was bounced
	__BLK_TA_REMAP,			// bio was remapped
	__BLK_TA_ABORT,			// request aborted
	__BLK_TA_DRV_DATA,		// binary driver data
	__BLK_TA_CGROUP = 1 << 8,
};

// Trace actions in full. Additionally, read or write is masked
#define BLK_TA_QUEUE		(__BLK_TA_QUEUE | BLK_TC_ACT(BLK_TC_QUEUE))
#define BLK_TA_BACKMERGE	(__BLK_TA_BACKMERGE | BLK_TC_ACT(BLK_TC_QUEUE))
#define BLK_TA_FRONTMERGE	(__BLK_TA_FRONTMERGE | BLK_TC_ACT(BLK_TC_QUEUE))
#define	BLK_TA_GETRQ		(__BLK_TA_GETRQ | BLK_TC_ACT(BLK_TC_QUEUE))
#define	BLK_TA_SLEEPRQ		(__BLK_TA_SLEEPRQ | BLK_TC_ACT(BLK_TC_QUEUE))
#define	BLK_TA_REQUEUE		(__BLK_TA_REQUEUE | BLK_TC_ACT(BLK_TC_REQUEUE))
#define BLK_TA_ISSUE		(__BLK_TA_ISSUE | BLK_TC_ACT(BLK_TC_ISSUE))
#define BLK_TA_COMPLETE		(__BLK_TA_COMPLETE| BLK_TC_ACT(BLK_TC_COMPLETE))
#define BLK_TA_PLUG		(__BLK_TA_PLUG | BLK_TC_ACT(BLK_TC_QUEUE))
#define BLK_TA_UNPLUG_IO	(__BLK_TA_UNPLUG_IO | BLK_TC_ACT(BLK_TC_QUEUE))
#define BLK_TA_UNPLUG_TIMER	(__BLK_TA_UNPLUG_TIMER | BLK_TC_ACT(BLK_TC_QUEUE))
#define BLK_TA_INSERT		(__BLK_TA_INSERT | BLK_TC_ACT(BLK_TC_QUEUE))
#define BLK_TA_SPLIT		(__BLK_TA_SPLIT)
#define BLK_TA_BOUNCE		(__BLK_TA_BOUNCE)
#define BLK_TA_REMAP		(__BLK_TA_REMAP | BLK_TC_ACT(BLK_TC_QUEUE))
#define BLK_TA_ABORT		(__BLK_TA_ABORT | BLK_TC_ACT(BLK_TC_QUEUE))
#define BLK_TA_DRV_DATA		(__BLK_TA_DRV_DATA | BLK_TC_ACT(BLK_TC_DRV_DATA))

#define BLK_TN_PROCESS		(__BLK_TN_PROCESS | BLK_TC_ACT(BLK_TC_NOTIFY))
#define BLK_TN_TIMESTAMP	(__BLK_TN_TIMESTAMP | BLK_TC_ACT(BLK_TC_NOTIFY))
#define BLK_TN_MESSAGE		(__BLK_TN_MESSAGE | BLK_TC_ACT(BLK_TC_NOTIFY))

#define BLK_IO_TRACE_MAGIC	0x65617400
#define BLK_IO_TRACE_VERSION	0x07

#pragma pack(1)
struct blk_io_trace {
	__u32 magic;		// MAGIC << 8 | version
	__u32 sequence;		// event number
	__u64 time;		// in nanoseconds
	__u64 sector;		// disk offset
	__u32 bytes;		// transfer length
	__u32 action;		// what happened
	__u32 pid;		// who did it
	__u32 device;		// device identifier (dev_t)
	__u32 cpu;		// on what cpu did it happen
	__u16 error;		// completion error
	__u16 pdu_len;		// length of data after this trace
};
*/
import "C"

import (
	"log"
	"net"
)

func main() {
	// Listen on TCP port 2000 on all available unicast and
	// anycast IP addresses of the local system.
	l, err := net.Listen("tcp", ":2000")
	if err != nil {
		log.Fatal(err)
	}
	defer l.Close()
	for {
		// Wait for a connection.
		conn, err := l.Accept()
		if err != nil {
			log.Fatal(err)
		}
		// Handle the connection in a new goroutine.
		// The loop then returns to accepting, so that
		// multiple connections may be served concurrently.
		go func(c net.Conn) {
			c.Close()
		}(conn)
	}
}
